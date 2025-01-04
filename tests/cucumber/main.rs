mod cli_world;

use crate::cli_world::CliWorld;
use cucumber::World;

#[tokio::main]
async fn main() {
    let _ = CliWorld::cucumber()
        .repeat_skipped()
        .fail_on_skipped()
        .run_and_exit("tests/features/cli")
        .await;
}
