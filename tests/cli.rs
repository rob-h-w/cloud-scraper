use assert_cmd::Command;

const BIN: &str = "cloud_scraper";

// #[test]
fn run_cli_env_debug() {
    Command::cargo_bin(BIN)
        .unwrap()
        .env("RUST_LOG", "debug")
        .assert()
        .success()
        .stderr(predicates::str::contains("Reading config..."))
        .stderr(predicates::str::contains("Checking config..."))
        .stderr(predicates::str::contains("Constructing engine..."))
        .stderr(predicates::str::contains("Starting engine"));
}
