use assert_cmd::Command;
use chrono::{TimeDelta, Utc};
use lazy_static::lazy_static;
use std::sync::Mutex;
use tokio_test::assert_ok;

const BIN: &str = "cloud_scraper";

lazy_static! {
    // Ensure that only one test runs at a time.
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

#[test]
fn run_cli_env_debug() {
    let _lock = MUTEX.lock().unwrap();
    Command::cargo_bin(BIN)
        .unwrap()
        .env("RUST_LOG", "debug")
        .arg("serve")
        .arg("--exit-after=1")
        .arg("--port=8080")
        .assert()
        .success()
        .stderr(predicates::str::contains("Reading config..."))
        .stderr(predicates::str::contains("Checking config..."))
        .stderr(predicates::str::contains("Constructing engine..."))
        .stderr(predicates::str::contains("Starting engine"));
}

#[test]
fn run_env_debug_with_empty_config() {
    let _lock = MUTEX.lock().unwrap();
    Command::cargo_bin(BIN)
        .unwrap()
        .env("RUST_LOG", "debug")
        .arg("serve")
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
    let _lock = MUTEX.lock().unwrap();
    let start = Utc::now();
    Command::cargo_bin(BIN)
        .unwrap()
        .env("RUST_LOG", "debug")
        .arg("serve")
        .arg("--config=tests/fixtures/empty_config.yaml")
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

#[test]
fn run_root_password_subcommand() {
    let _lock = MUTEX.lock().unwrap();
    let mut cmd = Command::cargo_bin(BIN).expect("Failed to build command");
    cmd.env("RUST_LOG", "debug")
        .arg("root-password")
        .write_stdin("password\n")
        .assert()
        .stdout(predicates::str::contains("Input root password:"))
        .success();

    assert_ok!(std::fs::metadata("root_password.yaml"));

    std::fs::remove_file("root_password.yaml").expect("Failed to delete file");
}
