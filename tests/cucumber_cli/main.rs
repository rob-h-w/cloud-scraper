mod cli_world;

use crate::cli_world::CliWorld;
use cucumber::{World, WriterExt};

#[tokio::main]
async fn main() {
    run_all_of(vec![
        "tests/features/config.feature",
        "tests/features/serve.feature",
    ])
    .await;
}

async fn run_all_of(features: Vec<&str>) {
    for feature in features {
        let _ = CliWorld::cucumber()
            .run_and_exit(feature)
            .await
            .fail_on_skipped();
    }
}
