use crate::shared::test_config;
use cloud_scraper::domain::Config;
use cucumber::gherkin::Step;
use cucumber::{given, then, when, World};
use derive_getters::Getters;
use regex::RegexBuilder;
use std::cmp::PartialEq;
use std::collections::HashMap;
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

pub(crate) trait ResponseExpecter {
    fn expect_response<'a, 'b>(&'a self, world: &'b CliWorld) -> &'b HttpResponse;
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct RequestMethodAndUrl {
    method: String,
    url: String,
}

impl ResponseExpecter for RequestMethodAndUrl {
    fn expect_response<'a, 'b>(&'a self, world: &'b CliWorld) -> &'b HttpResponse {
        world
            .http_request_shortcuts
            .get(self)
            .unwrap_or_else(|| panic!("Could not get a request for shortcut {:?}", self))
            .expect_response(world)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct GetRequest {
    url: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct PostRequest {
    url: String,
    body: String,
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) enum HttpRequest {
    GET(GetRequest),
    POST(PostRequest),
}

impl ResponseExpecter for HttpRequest {
    fn expect_response<'a, 'b>(&'a self, world: &'b CliWorld) -> &'b HttpResponse {
        world.http_transactions.expect(self)
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(crate) struct HttpResponse {
    body: String,
    headers: Vec<(String, String)>,
    status_code: u16,
}

#[derive(Clone, Default, Eq, PartialEq)]
struct RequestResponseMap {
    map: HashMap<HttpRequest, Option<Result<HttpResponse, String>>>,
}

impl RequestResponseMap {
    pub(crate) fn insert(
        &mut self,
        request: &HttpRequest,
        response: &Option<Result<HttpResponse, String>>,
    ) {
        self.map.insert(request.clone(), response.clone());
    }

    pub(crate) fn expect(&self, request: &HttpRequest) -> &HttpResponse {
        self.get(request)
            .expect(&format!("No response for request {:?}", request))
            .as_ref()
            .unwrap_or_else(|e| panic!("Error getting response for request {:?}: {}", request, e))
    }

    pub(crate) fn get(&self, request: &HttpRequest) -> Option<&Result<HttpResponse, String>> {
        match self.map.get(request) {
            Some(response) => response.as_ref(),
            None => None,
        }
    }
}

impl Debug for RequestResponseMap {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RequestResponseMap").finish()
    }
}

#[derive(Debug, Default, Getters, World)]
#[world(init = Self::new)]
pub(crate) struct CliWorld {
    args: Vec<String>,
    command: Option<String>,
    environment_variables: Vec<(String, String)>,
    http_requests: Vec<HttpRequest>,
    http_request_shortcuts: HashMap<RequestMethodAndUrl, HttpRequest>,
    http_transactions: RequestResponseMap,
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
            environment_variables: Default::default(),
            http_requests: Default::default(),
            http_request_shortcuts: Default::default(),
            http_transactions: Default::default(),
            input_sequence: Default::default(),
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

    pub(crate) fn expect_response<T>(&self, request: &T) -> &HttpResponse
    where
        T: ResponseExpecter,
    {
        request.expect_response(self)
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

    pub(crate) async fn send_request(&mut self, request: &HttpRequest) {
        self.trigger().await;
        self.http_requests.push(request.clone());

        let request_method_and_url = match request {
            HttpRequest::GET(get) => RequestMethodAndUrl {
                method: "GET".to_string(),
                url: get.url.clone(),
            },
            HttpRequest::POST(post) => RequestMethodAndUrl {
                method: "POST".to_string(),
                url: post.url.clone(),
            },
        };

        self.http_request_shortcuts
            .insert(request_method_and_url, request.clone());

        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .expect("Error building reqwest client");

        let result = match request {
            HttpRequest::GET(ref get_request) => client
                .get(&get_request.url)
                .send()
                .await
                .map_err(|e| e.to_string()),
            HttpRequest::POST(ref post_request) => client
                .post(&post_request.url)
                .body(post_request.body.clone())
                .send()
                .await
                .map_err(|e| e.to_string()),
        };
        let result = match result {
            Ok(response) => {
                let status_code = response.status().as_u16();
                let headers = response
                    .headers()
                    .iter()
                    .map(|(k, v)| (k.to_string(), v.to_str().unwrap().to_string()))
                    .collect();
                let body = response.text().await.unwrap();

                Ok(HttpResponse {
                    body,
                    headers,
                    status_code,
                })
            }
            Err(e) => Err(e),
        };

        self.http_transactions.insert(&request, &Some(result));
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

#[when(regex = r#"^I request "GET" "([a-zA-Z0-9.-/:]+)"$"#)]
pub(crate) async fn i_request_get(cli_world: &mut CliWorld, url: String) {
    cli_world
        .send_request(&HttpRequest::GET(GetRequest { url }))
        .await;
}

#[when(regex = r#"^I request "POST" "([a-zA-Z0-9.-/:]+)" with body:$"#)]
pub(crate) async fn i_request_post_with_body(cli_world: &mut CliWorld, url: String, step: &Step) {
    cli_world
        .send_request(&HttpRequest::POST(PostRequest {
            body: step.docstring.as_ref().expect("no docstring found").clone(),
            url,
        }))
        .await;
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

fn regex_from_step(step: &Step) -> (regex::Regex, String) {
    let regex_string = step
        .docstring
        .as_ref()
        .expect("No docstring in Cucumber step")
        .replace("\n", "\\n");
    (
        RegexBuilder::new(&regex_string)
            .multi_line(true)
            .dot_matches_new_line(true)
            .build()
            .expect(&format!("Error parsing regex {}", regex_string)),
        regex_string,
    )
}

fn assert_step_regex_matches(step: &Step, actual: &str) {
    let (re, regex_string) = regex_from_step(step);
    assert!(
        re.is_match(actual),
        "Could not match regex\n{}\nto\n{}",
        regex_string,
        actual
    );
}

#[then(regex = r#"^the stderr should have matched:$"#)]
pub(crate) async fn the_stderr_should_have_matched(cli_world: &mut CliWorld, step: &Step) {
    cli_world.trigger().await;
    let output = cli_world.output.clone().expect("Output not set");
    let stderr = String::from_utf8(output.stderr).expect("Error parsing stdout");
    assert_step_regex_matches(step, &stderr);
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

#[when(regex = r#"^I wait ([\d]+) seconds?$"#)]
#[then(regex = r#"^wait ([\d]+) seconds?$"#)]
pub(crate) async fn after_seconds(cli_world: &mut CliWorld, seconds: u64) {
    cli_world.trigger().await;
    tokio::time::sleep(Duration::from_secs(seconds)).await;
}

#[then(expr = "after the process ends")]
pub(crate) async fn after_the_process_ends(cli_world: &mut CliWorld) {
    cli_world.finish().await;
}

#[then(
    regex = r#"^the request "(GET|POST)" "([a-zA-Z0-9.-/:]+)" should return a status code of (\d+)$"#
)]
pub(crate) async fn the_request_should_return_a_status_code_of(
    cli_world: &mut CliWorld,
    method: String,
    url: String,
    status_code: u16,
) {
    cli_world.trigger().await;
    let response = cli_world.expect_response(&RequestMethodAndUrl { method, url });

    assert_eq!(response.status_code, status_code, "Status code mismatch");
}

#[then(
    regex = r#"^the request "(GET|POST)" "([a-zA-Z0-9.-/:]+)" should return a response matching:$"#
)]
pub(crate) async fn the_request_should_return_a_response_matching(
    cli_world: &mut CliWorld,
    method: String,
    url: String,
    step: &Step,
) {
    cli_world.trigger().await;
    let response = cli_world.expect_response(&RequestMethodAndUrl { method, url });
    assert_step_regex_matches(step, &response.body);
}

#[then(
    regex = r#"^the request "(GET|POST)" "([a-zA-Z0-9.-/:]+)" should return a header "([a-zA-Z_0-9-]+)" with "([a-zA-Z_0-9:*/+-]+)"$"#
)]
pub(crate) async fn the_request_should_return_a_header_value_of(
    cli_world: &mut CliWorld,
    method: String,
    url: String,
    header: String,
    value: String,
) {
    cli_world.trigger().await;
    let response = cli_world.expect_response(&RequestMethodAndUrl { method, url });
    let (_header, actual_value) = response
        .headers
        .iter()
        .find(|(k, _)| k == &header)
        .expect("Header not found");
    assert_eq!(&value, actual_value, "Header value mismatch");
}

async fn is_port_open(addr: &str) -> bool {
    match TcpListener::bind(addr).await {
        Ok(_) => false,
        _ => true,
    }
}
