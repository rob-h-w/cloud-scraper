use cloud_scraper::domain::{Config, DomainConfig};
use cucumber::gherkin::Step;
use cucumber::{given, then, when, World};
use derive_getters::Getters;
use std::cmp::PartialEq;
use std::fmt::Debug;
use std::path::PathBuf;
use std::process::Output;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;
use tokio_test::assert_ok;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum InputType {
    Kill,
    String(String),
}

#[derive(Debug, Default, Getters, World)]
#[world(init = Self::new)]
pub(crate) struct CliWorld {
    args: Vec<String>,
    command: Option<String>,
    input_sequence: Vec<InputType>,
    output: Option<Output>,
}

impl CliWorld {
    pub(crate) fn new() -> Self {
        Self {
            args: Vec::new(),
            command: None,
            input_sequence: Vec::new(),
            output: None,
        }
    }

    pub(crate) async fn retrieve_output(&mut self) -> Output {
        if self.output.is_some() {
            return self.output.clone().expect("Output not set");
        }

        let command = self.command.clone().expect("Command not set");
        let command = PathBuf::new().join("target/debug").join(command);
        let stdin_wanted = self
            .input_sequence
            .first()
            .is_some_and(|i| i != &InputType::Kill);

        let mut child = Command::new(command)
            .stdin(std::process::Stdio::piped())
            .args(self.args.clone())
            .spawn()
            .expect("Error spawning command");

        if stdin_wanted {
            let mut stdin = child.stdin.take().expect("Error taking stdin");

            for input in self.input_sequence.clone() {
                match input {
                    InputType::Kill => {
                        child.start_kill().expect("Error killing command");
                        break;
                    }
                    InputType::String(string) => {
                        stdin
                            .write((string + "\n").as_bytes())
                            .await
                            .expect("Error writing to stdin");
                    }
                }
            }
        }

        let output = child
            .wait_with_output()
            .await
            .expect("Error waiting for command");

        self.output = Some(output);
        self.output.clone().expect("Output not set")
    }
}

#[given(regex = r#"no config file named "([\S ]+)""#)]
async fn no_config(_cli_world: &mut CliWorld, path: String) {
    if fs::try_exists(&path)
        .await
        .expect("Error checking file existence")
    {
        fs::remove_file(&path)
            .await
            .expect(&format!("Error removing {}", path));
    }
}

#[given("a test config")]
async fn a_config_file(_cli_world: &mut CliWorld) {
    fs::write(
        "config.yaml",
        serde_yaml::to_string(&test_config()).unwrap(),
    )
    .await
    .expect("Error writing config file");
}

fn test_config() -> Config {
    Config::with_all_properties(
        Some(DomainConfig::new("test.domain".to_string())),
        Some("user@test.domain".to_string()),
        None,
        Some(8080),
        None,
    )
}

#[when(regex = r#"^I run "([\S "]+)"$"#)]
pub(crate) async fn i_run(cli_world: &mut CliWorld, command: String) {
    let mut raw = command.split_ascii_whitespace();
    cli_world.command = Some(raw.next().expect("Error parsing command").to_string());
    cli_world.args = raw.map(|s| s.to_string()).collect();
}

#[when(regex = r#"I enter \"([\S ]*)\""#)]
pub(crate) async fn i_enter(cli_world: &mut CliWorld, input: String) {
    cli_world.input_sequence.push(InputType::String(input));
}

#[when(expr = "I kill the process")]
pub(crate) async fn i_kill_the_process(cli_world: &mut CliWorld) {
    cli_world.input_sequence.push(InputType::Kill);
}

#[then(regex = r#"^the file "([\S "]+)" should not exist$"#)]
pub(crate) async fn the_file_should_not_exist(cli_world: &mut CliWorld, path: String) {
    cli_world.retrieve_output().await;
    assert!(
        !std::fs::exists(&path).expect(&format!("Error checking {} existence", &path)),
        "File {} exists",
        &path
    );
}

#[then(regex = r#"^the file "([\S "]+)" should exist$"#)]
pub(crate) async fn the_file_should_exist(cli_world: &mut CliWorld, path: String) {
    cli_world.retrieve_output().await;
    assert!(
        std::fs::exists(&path).expect(&format!("Error checking {} existence", &path)),
        "File {} does not exist",
        &path
    );
}

#[then(regex = r#"^the exit code should be (\d+)$"#)]
pub(crate) async fn the_exit_code_should_be(cli_world: &mut CliWorld, expected_exit_code: i32) {
    let output = cli_world.retrieve_output().await;
    let actual = output.status.code().expect("Error getting exit code");
    assert_eq!(
        actual, expected_exit_code,
        "Exit code mismatch, expected {}, got {}",
        expected_exit_code, actual
    );
}

#[then(regex = r#"^the exit code should not be (\d+)$"#)]
pub(crate) async fn the_exit_code_should_not_be(cli_world: &mut CliWorld, expected_exit_code: i32) {
    let output = cli_world.retrieve_output().await;
    let actual = match output.status.code() {
        Some(code) => code,
        None => {
            return;
        }
    };
    assert_ne!(
        actual, expected_exit_code,
        "Exit code mismatch, expected {} to not equal {}",
        expected_exit_code, actual
    );
}

#[then(regex = r#"^the file "([\S "]+)" should be a valid config$"#)]
pub(crate) async fn the_file_should_be_a_valid_config(cli_world: &mut CliWorld, path: String) {
    cli_world.retrieve_output().await;
    let config = tokio::fs::read_to_string(&path)
        .await
        .expect("Error reading config file");
    let config = serde_yaml::from_str::<Config>(&config).expect("Error parsing config");
    assert_ok!(config.sanity_check());
}

#[then(regex = r#"^the file "([\S "]+)" should contain$"#)]
pub(crate) async fn the_file_should_contain(cli_world: &mut CliWorld, step: &Step, path: String) {
    cli_world.retrieve_output().await;
    let config = tokio::fs::read_to_string(&path)
        .await
        .expect("Error reading config file");

    assert_eq!(
        &config,
        step.docstring
            .as_ref()
            .expect("No docstring in Cucumber step"),
    );
}

#[then(expr = "the test config should be unchanged")]
pub(crate) async fn the_test_config_should_be_unchanged(_cli_world: &mut CliWorld) {
    let config = test_config();
    let actual = tokio::fs::read_to_string("config.yaml")
        .await
        .expect("Error reading config file");
    let actual = serde_yaml::from_str::<Config>(&actual).expect("Error parsing config");
    assert_eq!(actual, config, "Config mismatch");
}