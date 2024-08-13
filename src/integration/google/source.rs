use crate::domain::channel_handle::ChannelHandle;
use crate::domain::module_state::NamedModule;
use crate::domain::node::{InitReplier, Lifecycle, LifecycleAware, Manager};
use crate::integration::google::auth::get_authenticator;
use crate::integration::google::auth::web::get_config;
use crate::integration::google::Events;
use crate::server::OauthFlowDelegateFactory;
use derive_getters::Getters;
use log::{debug, error, info, trace};
use std::sync::Arc;
use tokio::sync::{mpsc, Semaphore};
use tokio::{join, task};
use Lifecycle::{Init, ReadConfig, Stop};

const SCOPES: [&str; 2] = [
    "https://www.googleapis.com/auth/docs",
    "https://www.googleapis.com/auth/tasks",
];

#[derive(Clone, Debug, Getters)]
pub struct Source {
    control_events: ChannelHandle<Events>,
    lifecycle_manager: Manager,
    flow_delegate_factory: OauthFlowDelegateFactory,
}

impl NamedModule for Source {
    fn name() -> &'static str {
        "google"
    }
}

impl Source {
    pub fn new(manager: &Manager, flow_delegate_factory: &OauthFlowDelegateFactory) -> Self {
        Self {
            control_events: ChannelHandle::new(),
            lifecycle_manager: manager.clone(),
            flow_delegate_factory: flow_delegate_factory.clone(),
        }
    }

    pub async fn run(&self) {
        let (load_sender, mut load_receiver) = mpsc::channel(1);
        let core_config = self.lifecycle_manager.core_config().clone();
        let semaphore = Arc::new(Semaphore::new(1));
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Could not acquire semaphore");
        let flow_delegate_factory = self.flow_delegate_factory.clone();
        let task = task::spawn(async move {
            drop(permit);

            loop {
                let installed_flow_delegate = flow_delegate_factory.get_installed_flow_delegate();
                match load_receiver.recv().await {
                    Some(_) => {
                        info!("Loading google source");
                    }
                    None => {
                        error!("Channel closed");
                        break;
                    }
                }
                let application_secret = if let Some(config) = get_config().await {
                    config.to_application_secret(&core_config)
                } else {
                    continue;
                };
                let authenticator =
                    match get_authenticator(installed_flow_delegate, &application_secret).await {
                        Ok(authenticator) => authenticator,
                        Err(e) => {
                            error!("Error while getting authenticator: {:?}", e);
                            continue;
                        }
                    };

                let token = authenticator.token(&SCOPES).await;
                debug!("Token: {:?}", token);
            }
        });

        let _permit = semaphore
            .acquire()
            .await
            .expect("Could not acquire semaphore");

        macro_rules! send_load {
            () => {
                match load_sender.send(()).await {
                    Ok(_) => {
                        trace!("Sent load event");
                    }
                    Err(e) => {
                        error!("Error while sending event: {}", e);
                        break;
                    }
                }
            };
        }

        let task_abort_handle = task.abort_handle();
        let mut stop_event_receiver = self.lifecycle_manager.readonly().get_receiver();
        let lifetime_task = task::spawn(async move {
            loop {
                match stop_event_receiver.recv().await {
                    Ok(event) => match event {
                        Stop => {
                            task_abort_handle.abort();
                            break;
                        }
                        Init(event) => {
                            send_load!();
                            event.reply_to_init_with((), "google_source").await
                        }
                        ReadConfig(_) => {
                            if event.is_this::<Source>() {
                                send_load!();
                            }
                        }
                        Lifecycle::Redirect(_, _) => {
                            trace!("Redirect event ignored");
                        }
                    },
                    Err(e) => {
                        error!("Error while receiving event: {}", e);
                    }
                }
            }
        });

        let (_task_result, _stop_result) = join!(task, lifetime_task);
    }
}
