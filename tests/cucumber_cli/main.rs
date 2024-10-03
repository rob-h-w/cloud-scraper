mod cli_world;

use crate::cli_world::CliWorld;
use cucumber::{World, WriterExt};

#[tokio::main]
async fn main() {
    let _ = CliWorld::cucumber()
        .run("tests/features/config.feature")
        .await
        .fail_on_skipped();
}
