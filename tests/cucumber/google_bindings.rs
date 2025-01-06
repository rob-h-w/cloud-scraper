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

#[then(regex = r#"it waits.*"#)]
async fn it_waits(world: &mut GoogleWorld) {
    assert!(world.run_result().await.timed_out(), "run did not wait");
}
