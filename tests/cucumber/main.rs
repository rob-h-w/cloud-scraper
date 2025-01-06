extern crate core;

mod cli_world;
mod google_bindings;
mod google_world;
mod shared;

use crate::cli_world::CliWorld;
use crate::google_world::GoogleWorld;
use cucumber::World;

#[tokio::main]
async fn main() {
    let _ = CliWorld::cucumber()
        .repeat_skipped()
        .fail_on_skipped()
        .run_and_exit("tests/features/cli")
        .await;
    let _ = GoogleWorld::cucumber()
        .repeat_skipped()
        .fail_on_skipped()
        .run_and_exit("tests/features/google")
        .await;
}
