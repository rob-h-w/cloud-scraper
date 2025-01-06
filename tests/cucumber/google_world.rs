use cloud_scraper::domain::node::{LifecycleChannelHandle, Manager};
use cloud_scraper::domain::{one_shot, Config};
use cloud_scraper::integration::google::Source;
use cloud_scraper::server::WebEventChannelHandle;
use cucumber::World;
use derive_getters::Getters;
use std::fmt::Debug;
use std::future::Future;
use std::sync::{Arc, Once};
use tokio::sync::{OnceCell, OwnedSemaphorePermit, Semaphore};
use tokio::{join, select};

#[derive(Clone, Debug, Getters)]
pub(crate) struct RunResult {
    replied_to_init: bool,
    semaphore_released: bool,
    timed_out: bool,
}

impl RunResult {
    fn new() -> Self {
        Self {
            replied_to_init: false,
            semaphore_released: false,
            timed_out: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Init,
    Run,
    Stop,
}

#[derive(Debug, Getters, World)]
#[world(init = Self::new)]
pub(crate) struct GoogleWorld {
    actions: Vec<Action>,
    execute_once: OnceCell<RunResult>,
    #[getter(skip)]
    manager: Option<Manager>,
    #[getter(skip)]
    source: Option<Arc<Source>>,
    source_once: Once,
    web_channel_handle: WebEventChannelHandle,
}

impl GoogleWorld {
    pub(crate) fn call_run(&mut self) {
        self.actions.push(Action::Run);
    }

    pub(crate) fn send_init(&mut self) {
        self.actions.push(Action::Init);
    }

    pub(crate) fn send_stop(&mut self) {
        self.actions.push(Action::Stop);
    }

    pub(crate) fn set_config(&mut self, config: &Arc<Config>) {
        self.manager = Some(Manager::new(config, LifecycleChannelHandle::new()));
    }
}

impl GoogleWorld {
    pub(crate) async fn run_result(&mut self) -> RunResult {
        if self.execute_once.initialized() {
            return self.execute_once.get().unwrap().clone();
        }

        async fn test<'a>(
            permit: OwnedSemaphorePermit,
            semaphore: &'a Arc<Semaphore>,
            source: &'a Arc<Source>,
        ) {
            drop(permit);
            source
                .run(semaphore.clone().acquire_owned().await.unwrap())
                .await;
        }

        async fn wait_for_timeout<'a, T>(future: T) -> bool
        where
            T: Future<Output = ()> + 'a,
        {
            select! {
                _ = future => {
                    false
                }
                _ = tokio::time::sleep(tokio::time::Duration::from_secs(1)) => {
                    true
                }
            }
        }

        async fn runner<'a>(
            actions: &Vec<Action>,
            manager: &mut Manager,
            test_semaphore: &Arc<Semaphore>,
        ) -> RunResult {
            let mut run_result = RunResult::new();

            for action in actions {
                match action {
                    Action::Init => {
                        let (sender, mut receiver) = one_shot::<()>();
                        manager.send_init(sender).unwrap();
                        receiver.recv().await.unwrap();
                        run_result.replied_to_init = true;
                    }
                    Action::Run => {
                        // Wait until the test is actually executing.
                        let _ = test_semaphore.acquire().await.unwrap();
                    }
                    Action::Stop => {
                        manager.send_stop().unwrap();
                    }
                }
            }

            run_result
        }

        let actions = self.actions.clone();
        let semaphore = Arc::new(Semaphore::new(1));
        let test_semaphore = Arc::new(Semaphore::new(1));
        let test_permit = test_semaphore.clone().acquire_owned().await.unwrap();
        let source = self.source().clone();
        let mut manager = self.manager().clone();

        let run_result = self
            .execute_once
            .get_or_init(|| async move {
                let test = Box::pin(test(test_permit, &semaphore, &source));
                let runner = runner(&actions, &mut manager, &test_semaphore);
                let race = wait_for_timeout(test);
                let (mut run_result, timed_out) = join!(runner, race);

                run_result.timed_out = timed_out;
                run_result.semaphore_released = semaphore.available_permits() == 1;

                run_result
            })
            .await
            .clone();

        // Send a stop in case there are any tasks still running.
        let _ = self.manager().clone().send_stop();

        run_result
    }
}

impl GoogleWorld {
    pub(crate) fn new() -> Self {
        Self {
            actions: Vec::new(),
            execute_once: OnceCell::new(),
            manager: None,
            source: None,
            source_once: Once::new(),
            web_channel_handle: WebEventChannelHandle::new(),
        }
    }

    fn manager(&self) -> &Manager {
        self.manager.as_ref().unwrap()
    }

    fn source(&mut self) -> Arc<Source> {
        self.source_once.call_once(|| {
            let source = Source::new(&self.manager.as_ref().unwrap(), &self.web_channel_handle);

            self.source.replace(Arc::new(source));
        });

        self.source.as_mut().unwrap().clone()
    }
}
