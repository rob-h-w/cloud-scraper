@serial
Feature: Serve subcommand

  Scenario: Serve without root password
    Given no file named "root_password.yaml"
    When I run "cloud_scraper serve --port=8080"
    Then the stderr should have matched:
    """Root password not set"""
    And the exit code should not be 0

  Scenario: Serve env debug
    Given an environment variable "RUST_LOG" with the value "debug"
    Given no file named "config.yaml"
    Given no file named "state/google/config.yaml"
    Given a file named "root_password.yaml" containing:
    """"""
    When I run "cloud_scraper serve --exit-after=1 --port=8080"
    Then the stderr should have matched:
    """\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::main_impl\] Reading cli input\.\.\.
\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::main_impl\] Checking root password\.\.\.
\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::main_impl\] Reading config\.\.\.
\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::main_impl\] Checking config\.\.\.
\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::main_impl\] Constructing server\.\.\.
\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::main_impl\] Constructing engine\.\.\.
\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::main_impl\] Starting engine
    """
    And the stderr should have matched:
    """\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::integration::google::auth::web\] Root: "state/google".*
\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::integration::google::auth::web\] Config path: "state/google/config\.yaml".*
\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::integration::google::auth::web\] Read result: Err\(Os \{ code: 2, kind: NotFound, message: "No such file or directory" \}\)
    """
    And the stderr should have matched:
    """\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z INFO  cloud_scraper::integration::google::source\] Loading google source
    """
    And the stderr should have matched:
    """\s*\[[\d]{4}-[\d]{2}-[\d]{2}T[\d]{2}:[\d]{2}:[\d]{2}Z DEBUG cloud_scraper::server::web_server\] Server listening on 0\.0\.0\.0:8080.*
    """
    And the exit code should be 0

  Scenario: Serve env debug with empty config
    Given an environment variable "RUST_LOG" with the value "debug"
    Given no file named "config.yaml"
    Given no file named "state/google/config.yaml"
    Given a file named "root_password.yaml" containing:
    """"""
    When I run "cloud_scraper serve --config=tests/fixtures/empty_config.yaml --port=8080"
    Then the stderr should have matched:
      """Reading config\.\.\.
.*Checking config\.\.\.
.*Constructing engine\.\.\.
.*Starting engine
      """
    And the exit code should be 0

  Scenario: Serve env debug with empty config and a cli exit override parameter
    Given an environment variable "RUST_LOG" with the value "debug"
    Given no file named "config.yaml"
    Given no file named "state/google/config.yaml"
    Given a file named "root_password.yaml" containing:
    """"""
    When I run "cloud_scraper serve --config=tests/fixtures/empty_config.yaml --exit-after=0"
    Then the stderr should have matched:
      """Reading config\.\.\.
.*Checking config\.\.\.
.*Constructing engine\.\.\.
.*Starting engine
        """

  Scenario: Start the service without HTTPS, opens the configured port and serves the API
    Given no file named "config.yaml"
    Given no file named "root_password.yaml"
    Given a file named "root_password.yaml" containing:
    """hash: b4f27c30d7530f6f8d9edca87a86c867d9a1d537
salt: eyF8Ak6G48ZrRBs0
"""
    Given a file named "config.yaml" containing:
    """domain_config:
  url: http://localhost:4321/
email: email@test.scenario.domain
"""
    When I start "cloud_scraper serve --exit-after=2"
    Then wait 1 second
    And the port 4321 should be open
    And after the process ends
    And the port 4321 should be closed
    And the exit code should be 0

  Scenario: Login page is served
    Given a file named "root_password.yaml" containing:
    """hash: b4f27c30d7530f6f8d9edca87a86c867d9a1d537
salt: eyF8Ak6G48ZrRBs0
"""
    Given a file named "config.yaml" containing:
    """domain_config:
  url: http://localhost:4321/
email: email@test.scenario.domain
"""
    When I start "cloud_scraper serve --exit-after=2"
    When I request "GET" "http://localhost:4321/login"
    Then the request "GET" "http://localhost:4321/login" should return a status code of 200
    And the request "GET" "http://localhost:4321/login" should return a response matching:
    """.*Admin Login.*
    """
