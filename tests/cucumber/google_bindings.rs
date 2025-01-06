use crate::google_world::GoogleWorld;
use crate::shared::test_config;
use cucumber::{given, then, when};
use std::sync::Arc;

#[given("a test config")]
fn a_config_file(world: &mut GoogleWorld) {
    let config = Arc::new(test_config());
    world.set_config(&config);
}

#[when("I call run")]
fn i_call_run(world: &mut GoogleWorld) {
    world.call_run();
}

#[when("I send_init")]
fn i_send_init(world: &mut GoogleWorld) {
    world.send_init();
}

#[then("it releases the semaphore")]
async fn it_releases_the_semaphore(world: &mut GoogleWorld) {
    assert!(
        world.run_result().await.semaphore_released(),
        "run did not release semaphore"
    );
}

#[then(regex = r#"it waits.*"#)]
async fn it_waits(world: &mut GoogleWorld) {
    assert!(world.run_result().await.timed_out(), "run did not wait");
}

#[then("it replies to init with ()")]
async fn it_replies_to_init_with(world: &mut GoogleWorld) {
    assert!(
        world.run_result().await.replied_to_init(),
        "run did not reply to init with ()"
    );
}
