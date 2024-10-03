use cucumber::{given, then, when, World};
use derive_getters::Getters;
use std::path::PathBuf;
use std::process::Output;
use tokio::io::AsyncWriteExt;

#[derive(Clone, Debug)]
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
    fn new() -> Self {
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

        println!("Running command: {:?}", command);

        let mut child = tokio::process::Command::new(command)
            .args(self.args.clone())
            .spawn()
            .expect("Error spawning command");

        if !self.input_sequence.is_empty() {
            let stdin = child.stdin.as_mut().expect("Error getting stdin");

            for input in self.input_sequence.clone() {
                match input {
                    InputType::Kill => {
                        child.start_kill().expect("Error killing command");
                        break;
                    }
                    InputType::String(string) => {
                        stdin
                            .write(string.as_bytes())
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

#[given("no config")]
fn no_config(_: &mut CliWorld) {
    if std::fs::exists("config.yaml").expect("Error checking file existence") {
        std::fs::remove_file("config.yaml").expect("Error removing config file");
    }
}

#[when(regex = r#"^I run "([\S "]+)"$"#)]
pub(crate) async fn i_run(cli_world: &mut CliWorld, command: String) {
    let mut raw = command.split_ascii_whitespace();
    cli_world.command = Some(raw.next().expect("Error parsing command").to_string());
    cli_world.args = raw.map(|s| s.to_string()).collect();
}

#[when(expr = "I enter \"{word}\"")]
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
