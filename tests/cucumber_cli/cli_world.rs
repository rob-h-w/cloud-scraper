use cloud_scraper::domain::{Config, DomainConfig};
use cucumber::gherkin::Step;
use cucumber::{given, then, when, World};
use derive_getters::Getters;
use regex::RegexBuilder;
use std::cmp::PartialEq;
use std::fmt::Debug;
use std::future::Future;
use std::io;
use std::path::PathBuf;
use std::pin::Pin;
use std::process::Output;
use std::time::Duration;
use tokio::fs;
use tokio::io::AsyncWriteExt;
use tokio::net::TcpListener;
use tokio::process::Command;
use tokio_test::assert_ok;

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum InputType {
    Kill,
    String(String),
}

struct PinnedBoxedFutureWrapper {
    inner: Pin<Box<dyn Future<Output = io::Result<Output>>>>,
}

impl PinnedBoxedFutureWrapper {
    pub(crate) fn new(inner: Pin<Box<dyn Future<Output = io::Result<Output>>>>) -> Self {
        Self { inner }
    }

    async fn wait(self) -> io::Result<Output> {
        self.inner.await
    }
}

impl Debug for PinnedBoxedFutureWrapper {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PinnedBoxedFutureWrapper").finish()
    }
}

#[derive(Debug, Default, Getters, World)]
#[world(init = Self::new)]
pub(crate) struct CliWorld {
    args: Vec<String>,
    command: Option<String>,
    environment_variables: Vec<(String, String)>,
    input_sequence: Vec<InputType>,
    output: Option<Output>,
    output_future: Option<PinnedBoxedFutureWrapper>,
    wait_for_process_to_terminate: bool,
}

impl CliWorld {
    pub(crate) fn new() -> Self {
        Self {
            args: Vec::new(),
            command: None,
            environment_variables: vec![],
            input_sequence: Vec::new(),
            output: None,
            output_future: None,
            wait_for_process_to_terminate: false,
        }
    }

    pub(crate) async fn expect_output(&mut self) -> Output {
        self.finish().await;
        self.output
            .clone()
            .expect("Output not set - did the command finish?")
    }

    pub(crate) async fn finish(&mut self) {
        if self.output.is_some() {
            return;
        }

        self.start_process().await;
        self.output = Some(
            self.output_future
                .take()
                .expect("Output future not set")
                .wait()
                .await
                .expect("Error waiting for command"),
        );
    }

    pub(crate) async fn start_process(&mut self) {
        if self.output_future.is_some() || self.output.is_some() {
            return;
        }

        let command = self.command.clone().expect("Command not set");
        let command = PathBuf::new().join("target/debug").join(command);

        let cmd = command.clone();
        let mut command = Command::new(cmd);
        let mut command = command
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped());
        command = command.args(self.args.clone());

        for (key, value) in self.environment_variables.clone() {
            command = command.env(key, value);
        }

        let mut child = command.spawn().expect("Error spawning command");

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

        self.output_future = Some(PinnedBoxedFutureWrapper::new(Box::pin(
            child.wait_with_output(),
        )));
    }

    pub(crate) async fn trigger(&mut self) {
        if self.wait_for_process_to_terminate {
            self.finish().await;
        } else {
            self.start_process().await;
        }
    }
}

#[given(regex = r#"no file named "([\S ]+)""#)]
async fn no_file(_cli_world: &mut CliWorld, path: String) {
    if fs::try_exists(&path)
        .await
        .expect("Error checking file existence")
    {
        fs::remove_file(&path)
            .await
            .expect(&format!("Error removing {}", path));
    }
}

#[given(regex = r#"a file named "([\S ]+)" containing:"#)]
async fn a_file_containing(_cli_world: &mut CliWorld, step: &Step, path: String) {
    if fs::try_exists(&path)
        .await
        .expect("Error checking file existence")
    {
        fs::remove_file(&path)
            .await
            .expect(&format!("Error removing {}", path));
    }

    fs::write(&path, step.docstring.as_ref().unwrap().as_bytes())
        .await
        .expect(&format!("Error writing to {}", path));
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
        Some(DomainConfig::new("http://test.domain:8080")),
        Some("user@test.domain".to_string()),
        None,
        None,
    )
}

#[given(regex = r#"an environment variable "([\S ]+)" with the value "([\S ]+)""#)]
fn set_environment_variable(cli_world: &mut CliWorld, key: String, value: String) {
    cli_world.environment_variables.push((key, value));
}

#[when(regex = r#"^I run "([\S "]+)"$"#)]
pub(crate) async fn i_run(cli_world: &mut CliWorld, command: String) {
    let mut raw = command.split_ascii_whitespace();
    cli_world.command = Some(raw.next().expect("Error parsing command").to_string());
    cli_world.args = raw.map(|s| s.to_string()).collect();
    cli_world.wait_for_process_to_terminate = true;
}

#[when(regex = r#"^I start "([\S "]+)"$"#)]
pub(crate) async fn i_start(cli_world: &mut CliWorld, command: String) {
    let mut raw = command.split_ascii_whitespace();
    cli_world.command = Some(raw.next().expect("Error parsing command").to_string());
    cli_world.args = raw.map(|s| s.to_string()).collect();
    cli_world.wait_for_process_to_terminate = false;
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
    cli_world.trigger().await;
    assert!(
        !std::fs::exists(&path).expect(&format!("Error checking {} existence", &path)),
        "File {} exists",
        &path
    );
}

#[then(regex = r#"^the file "([\S "]+)" should exist$"#)]
pub(crate) async fn the_file_should_exist(cli_world: &mut CliWorld, path: String) {
    cli_world.trigger().await;
    assert!(
        std::fs::exists(&path).expect(&format!("Error checking {} existence", &path)),
        "File {} does not exist",
        &path
    );
}

#[then(regex = r#"^the exit code should be (\d+)$"#)]
pub(crate) async fn the_exit_code_should_be(cli_world: &mut CliWorld, expected_exit_code: i32) {
    let output = cli_world.expect_output().await;
    let actual = output.status.code().expect("Error getting exit code");
    assert_eq!(
        actual, expected_exit_code,
        "Exit code mismatch, expected {}, got {}",
        expected_exit_code, actual
    );
}

#[then(regex = r#"^the exit code should not be (\d+)$"#)]
pub(crate) async fn the_exit_code_should_not_be(cli_world: &mut CliWorld, expected_exit_code: i32) {
    let output = cli_world.expect_output().await;
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
    cli_world.trigger().await;
    let config = tokio::fs::read_to_string(&path)
        .await
        .expect("Error reading config file");
    let config = serde_yaml::from_str::<Config>(&config).expect("Error parsing config");
    assert_ok!(config.sanity_check());
}

#[then(regex = r#"^the file "([\S "]+)" should contain:$"#)]
pub(crate) async fn the_file_should_contain(cli_world: &mut CliWorld, step: &Step, path: String) {
    cli_world.trigger().await;
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

#[then(regex = r#"^the stdout should have been:$"#)]
pub(crate) async fn the_prompts_should_have_been(cli_world: &mut CliWorld, step: &Step) {
    cli_world.trigger().await;
    let output = cli_world.output.clone().expect("Output not set");
    let stdout = String::from_utf8(output.stdout).expect("Error parsing stdout");
    assert_eq!(
        &stdout,
        step.docstring
            .as_ref()
            .expect("No docstring in Cucumber step"),
    );
}

#[then(regex = r#"^the stderr should have matched:$"#)]
pub(crate) async fn the_stderr_should_have_been(cli_world: &mut CliWorld, step: &Step) {
    cli_world.trigger().await;
    let output = cli_world.output.clone().expect("Output not set");
    let stderr = String::from_utf8(output.stderr).expect("Error parsing stdout");
    let regex_string = step
        .docstring
        .as_ref()
        .expect("No docstring in Cucumber step")
        .replace("\n", "\\n");
    let re = RegexBuilder::new(&regex_string)
        .multi_line(true)
        .dot_matches_new_line(true)
        .build()
        .expect(&format!("Error parsing regex {}", regex_string));
    let captures = re.captures(&stderr);
    assert!(
        captures.is_some(),
        "Could not match regex\n{}\nto stderr\n{}",
        regex_string,
        stderr
    );
}

#[then(expr = "the test config should be unchanged")]
pub(crate) async fn the_test_config_should_be_unchanged(cli_world: &mut CliWorld) {
    cli_world.trigger().await;
    let config = test_config();
    let actual = tokio::fs::read_to_string("config.yaml")
        .await
        .expect("Error reading config file");
    let actual = serde_yaml::from_str::<Config>(&actual).expect("Error parsing config");
    assert_eq!(actual, config, "Config mismatch");
}

#[then(regex = r#"^the port ([\d]+) should be open$"#)]
pub(crate) async fn the_port_should_be_open(cli_world: &mut CliWorld, port: u16) {
    cli_world.trigger().await;
    let addr = format!("127.0.0.1:{port}");
    assert!(is_port_open(&addr).await, "Port {port} is not open");
}

#[then(regex = r#"^the port ([\d]+) should be closed"#)]
pub(crate) async fn the_port_should_be_closed(cli_world: &mut CliWorld, port: u16) {
    cli_world.trigger().await;
    let addr = format!("127.0.0.1:{port}");
    assert!(!is_port_open(&addr).await, "Port {port} is open");
}

#[then(regex = r#"^wait ([\d]+) seconds?$"#)]
pub(crate) async fn after_seconds(cli_world: &mut CliWorld, seconds: u64) {
    cli_world.trigger().await;
    tokio::time::sleep(Duration::from_secs(seconds)).await;
}

#[then(expr = "after the process ends")]
pub(crate) async fn after_the_process_ends(cli_world: &mut CliWorld) {
    cli_world.finish().await;
}

async fn is_port_open(addr: &str) -> bool {
    match TcpListener::bind(addr).await {
        Ok(_) => false,
        _ => true,
    }
}
