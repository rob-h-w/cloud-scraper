use cloud_scraper::domain::node::{LifecycleChannelHandle, Manager};
use cloud_scraper::domain::Config;
use cloud_scraper::integration::google::Source;
use cloud_scraper::server::WebEventChannelHandle;
use cucumber::World;
use derive_getters::Getters;
use std::sync::{Arc, Once};
use tokio::sync::{OnceCell, Semaphore};

#[derive(Clone, Debug, Getters)]
pub(crate) struct RunResult {
    timed_out: bool,
}

#[derive(Clone, Debug)]
enum Action {
    Run,
}

#[derive(Debug, Getters, World)]
#[world(init = Self::new)]
pub(crate) struct GoogleWorld {
    actions: Vec<Action>,
    execute_once: OnceCell<RunResult>,
    manager: Option<Manager>,
    semaphore: Arc<Semaphore>,
    #[getter(skip)]
    source: Option<Arc<Source>>,
    source_once: Once,
    web_channel_handle: WebEventChannelHandle,
}

impl GoogleWorld {
    pub(crate) fn new() -> Self {
        Self {
            actions: Vec::new(),
            execute_once: OnceCell::new(),
            manager: None,
            semaphore: Arc::new(Semaphore::new(1)),
            source: None,
            source_once: Once::new(),
            web_channel_handle: WebEventChannelHandle::new(),
        }
    }

    pub(crate) fn call_run(&mut self) {
        self.actions.push(Action::Run);
    }

    pub(crate) async fn run_result(&mut self) -> RunResult {
        let actions = self.actions.clone();
        let semaphore = self.semaphore().clone();
        let source = self.source().clone();
        let run_result = self
            .execute_once
            .get_or_init(|| async move {
                let mut run_result = RunResult { timed_out: false };
                for action in actions {
                    match action {
                        Action::Run => {
                            let permit = semaphore.clone().acquire_owned().await.unwrap();

                            run_result.timed_out = tokio::select! {
                                _ = source.run(permit) => {
                                    false
                                }
                                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                                    true
                                }
                            };
                        }
                    }
                }

                run_result
            })
            .await;

        run_result.clone()
    }

    pub(crate) fn set_config(&mut self, config: &Arc<Config>) {
        self.manager = Some(Manager::new(config, LifecycleChannelHandle::new()));
    }

    fn source(&mut self) -> Arc<Source> {
        self.source_once.call_once(|| {
            let source = Source::new(&self.manager.as_ref().unwrap(), &self.web_channel_handle);

            self.source.replace(Arc::new(source));
        });

        self.source.as_mut().unwrap().clone()
    }
}
