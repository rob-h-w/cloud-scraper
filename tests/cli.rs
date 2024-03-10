use assert_cmd::Command;
use chrono::{TimeDelta, Utc};

const BIN: &str = "cloud_scraper";

#[test]
fn run_cli_env_debug() {
    Command::cargo_bin(BIN)
        .unwrap()
        .env("RUST_LOG", "debug")
        .arg("--exit-after=1")
        .assert()
        .success()
        .stderr(predicates::str::contains("Reading config..."))
        .stderr(predicates::str::contains("Checking config..."))
        .stderr(predicates::str::contains("Constructing engine..."))
        .stderr(predicates::str::contains("Starting engine"));
}

#[test]
fn run_env_debug_with_empty_config() {
    Command::cargo_bin(BIN)
        .unwrap()
        .env("RUST_LOG", "debug")
        .arg("--config=tests/fixtures/empty_config.yaml")
        .assert()
        .success()
        .stderr(predicates::str::contains("Reading config..."))
        .stderr(predicates::str::contains("Checking config..."))
        .stderr(predicates::str::contains("Constructing engine..."))
        .stderr(predicates::str::contains("Starting engine"));
}

#[test]
fn run_env_debug_with_empty_config_cli_exit_override() {
    let start = Utc::now();
    Command::cargo_bin(BIN)
        .unwrap()
        .env("RUST_LOG", "debug")
        .arg("--config=tests/fixtures/config.yaml")
        .arg("--exit-after=0")
        .assert()
        .success()
        .stderr(predicates::str::contains("Reading config..."))
        .stderr(predicates::str::contains("Checking config..."))
        .stderr(predicates::str::contains("Constructing engine..."))
        .stderr(predicates::str::contains("Starting engine"));
    let end = Utc::now();
    assert!(end - start < TimeDelta::try_seconds(1).unwrap());
}
