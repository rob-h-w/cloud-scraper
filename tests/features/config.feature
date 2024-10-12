@serial
Feature: Config subcommand

  Scenario: Unrecognized subcommand
    When I run "cloud_scraper unknown_subcommand"
    Then the exit code should not be 0

  Scenario: Exiting does not generate a file
    Given no file named "config.yaml"
    When I run "cloud_scraper config"
    When I kill the process
    Then the file "config.yaml" should not exist
    And the exit code should not be 0

  Scenario: Entering email generates a file
    Given no file named "config-test.yaml"
    When I run "cloud_scraper config -c config-test.yaml"
    When I enter "n"
    When I enter "test@test.com"
    When I enter ""
    Then the file "config-test.yaml" should exist
    And the file "config-test.yaml" should be a valid config
    And the stdout should have been:
    """Would you like to configure a domain? (Y/n)
Please enter the email you'd like to use as the admin contact when requesting a certificate:
Please enter the port you'd like to use for serving HTTPS traffic (leave blank for 443):
Please enter the folder where site state will be stored (leave blank for .site):
    """
    And the file "config-test.yaml" should contain:
    """email: test@test.com
    """
    And the exit code should be 0

  Scenario: Entering a port generates a file
    Given no file named "config.yaml"
    When I run "cloud_scraper config"
    When I enter "n"
    When I enter ""
    When I enter "12345"
    Then the file "config.yaml" should exist
    And the file "config.yaml" should be a valid config
    And the file "config.yaml" should contain:
    """port: 12345
    """
    And the exit code should be 0

  Scenario: Entering a domain without an email for certificate retrieval does not generate a file
    Given no file named "config.yaml"
    When I run "cloud_scraper config"
    When I enter "y"
    When I enter "y"
    When I enter ""
    When I enter "test.scenario.domain"
    When I enter "2"
    When I enter "1 "
    When I enter ""
    Then the file "config.yaml" should not exist
    And the exit code should not be 0

  Scenario: Entering all data generates a file
    Given no file named "config.yaml"
    When I run "cloud_scraper config"
    When I enter "y"
    When I enter "email-1@domain.owner.contact"
    When I enter "email-2@domain.owner.contact"
    When I enter ""
    When I enter "test.scenario.domain"
    When I enter "y"
    When I enter "external.uri"
    When I enter "/path"
    When I enter "8080"
    When I enter " 3000   "
    When I enter "1"
    When I enter "email@test.scenario.domain"
    When I enter "12345"
    When I enter ".my_site_state_folder"
    Then the file "config.yaml" should exist
    And the file "config.yaml" should be a valid config
    And the file "config.yaml" should contain:
    """domain_config:
  builder_contacts:
  - email-1@domain.owner.contact
  - email-2@domain.owner.contact
  domain_name: test.scenario.domain
  external_uri_config:
    domain: external.uri
    path: /path
    port: 8080
  poll_attempts: 3000
  poll_interval_seconds: 1
email: email@test.scenario.domain
port: 12345
site_state_folder: .my_site_state_folder
    """
    And the exit code should be 0

  Scenario: Not replacing config respects the choice
    Given a test config
    When I run "cloud_scraper config"
    When I enter "n"
    Then the test config should be unchanged
    And the exit code should be 0

  Scenario: Defaulting to not replacing config respects the choice
    Given a test config
    When I run "cloud_scraper config"
    When I enter ""
    Then the test config should be unchanged
    And the exit code should be 0
