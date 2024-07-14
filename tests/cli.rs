use assert_cmd::Command;
use chrono::{TimeDelta, Utc};
use lazy_static::lazy_static;
use std::sync::Mutex;

const BIN: &str = "cloud_scraper";
const ROOT_PASSWORD_FILE: &str = "root_password.yaml";

lazy_static! {
    // Ensure that only one test runs at a time.
    static ref MUTEX: Mutex<()> = Mutex::new(());
}

#[test]
fn serve_without_root_password_fails() {
    let _lock = MUTEX.lock().unwrap();
    let _ = std::fs::remove_file(ROOT_PASSWORD_FILE);

    Command::cargo_bin(BIN)
        .unwrap()
        .env("RUST_LOG", "debug")
        .arg("serve")
        .arg("--port=8080")
        .assert()
        .failure()
        .stderr(predicates::str::contains("Root password not set"));
}

#[test]
fn serve_env_debug() {
    let _lock = MUTEX.lock().unwrap();
    std::fs::write(ROOT_PASSWORD_FILE, "").expect("Failed to write root password file");
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
    std::fs::remove_file(ROOT_PASSWORD_FILE).expect("Failed to delete root password file");
}

#[test]
fn serve_env_debug_with_empty_config() {
    let _lock = MUTEX.lock().unwrap();
    std::fs::write(ROOT_PASSWORD_FILE, "").expect("Failed to write root password file");
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
    std::fs::remove_file(ROOT_PASSWORD_FILE).expect("Failed to delete root password file");
}

#[test]
fn serve_env_debug_with_empty_config_cli_exit_override() {
    let _lock = MUTEX.lock().unwrap();
    std::fs::write(ROOT_PASSWORD_FILE, "").expect("Failed to write root password file");
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
    std::fs::remove_file(ROOT_PASSWORD_FILE).expect("Failed to delete root password file");
    let end = Utc::now();
    assert!(end - start < TimeDelta::try_seconds(1).unwrap());
}
