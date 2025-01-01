use crate::domain::module_state::NamedModule;
use crate::domain::node::{InitReplier, Lifecycle, Manager};
use crate::domain::oauth2::{extra_parameters, Client};
use crate::integration::google::auth::web::get_config;
use crate::integration::google::auth::DelegateBuilder;
use crate::integration::google::tasks::sync;
use crate::server::auth::get_token_path;
use crate::server::WebEventChannelHandle;
use derive_getters::Getters;
use log::{error, info, trace};
use std::any::TypeId;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{mpsc, OwnedSemaphorePermit, Semaphore};
use tokio::time::sleep;
use tokio::{join, task};
use Lifecycle::{Init, ReadConfig, Stop};

#[derive(Clone, Debug, Getters)]
pub struct Source {
    lifecycle_manager: Manager,
    web_channel_handle: WebEventChannelHandle,
}

impl NamedModule for Source {
    fn name() -> &'static str {
        "google"
    }
}

impl Source {
    pub fn new(manager: &Manager, web_channel_handle: &WebEventChannelHandle) -> Self {
        Self {
            lifecycle_manager: manager.clone(),
            web_channel_handle: web_channel_handle.clone(),
        }
    }

    pub async fn run(&self, google_permit: OwnedSemaphorePermit) {
        let (load_sender, mut load_receiver) = mpsc::channel(1);
        let core_config = self.lifecycle_manager.core_config().clone();
        let semaphore = Arc::new(Semaphore::new(1));
        let mut permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Could not acquire semaphore");
        let lifecycle_manager = self.lifecycle_manager.clone();
        let web_channel_handle = self.web_channel_handle.clone();

        let task = task::spawn(async move {
            drop(permit);
            let mut initialized = false;

            loop {
                if !initialized {
                    match load_receiver.recv().await {
                        Some(_) => {
                            info!("Loading google source");
                            initialized = true;
                        }
                        None => {
                            error!("Channel closed");
                            break;
                        }
                    }
                } else {
                    if load_receiver.is_closed() {
                        break;
                    }
                }

                let application_secret = if let Some(config) = get_config().await {
                    config.to_application_secret(&core_config)
                } else {
                    Self::wait_in_loop().await;
                    continue;
                };
                let token_path = match get_token_path::<Self>().await {
                    Ok(path) => path,
                    Err(e) => {
                        error!("Problem getting or creating the token path: {}", e);
                        continue;
                    }
                };
                let client = Client::new(
                    application_secret,
                    &extra_parameters!("access_type" => "offline"),
                    &lifecycle_manager,
                    &token_path,
                    &web_channel_handle,
                );
                loop {
                    if !load_receiver.is_empty() || load_receiver.is_closed() {
                        break;
                    }

                    let delegate = match DelegateBuilder::default().client(client.clone()).build() {
                        Ok(delegate) => delegate,
                        Err(e) => {
                            error!("Error while creating Google authentication delegate: {}", e);
                            break;
                        }
                    };
                    sync(delegate).await;
                    Self::wait_in_loop().await;
                }
            }
        });

        {
            let _permit = semaphore
                .acquire()
                .await
                .expect("Could not acquire semaphore");
        }

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

        permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Could not acquire semaphore");

        let task_abort_handle = task.abort_handle();
        let mut stop_event_receiver = self.lifecycle_manager.readonly().get_receiver();
        let lifetime_task = task::spawn(async move {
            drop(permit);
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
                        ReadConfig(type_id) => {
                            if type_id == TypeId::of::<Source>() {
                                send_load!();
                            }
                        }
                    },
                    Err(e) => {
                        error!("Error while receiving event: {}", e);
                    }
                }
            }
        });

        {
            let _permit = semaphore
                .acquire()
                .await
                .expect("Could not acquire semaphore");
        }

        drop(google_permit);

        let (_task_result, _stop_result) = join!(task, lifetime_task);
    }

    async fn wait_in_loop() {
        sleep(Duration::from_secs(10)).await;
    }
}
