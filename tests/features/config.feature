Feature: Config subcommand

  Scenario: Unrecognized subcommand
    When I run "cloud_scraper unknown_subcommand"
    Then the exit code should be 2

  @Serial
  Scenario: Exiting does not generate a file
    Given no config
    When I run "cloud_scraper config"
    When I enter "test@test.com"
    When I kill the process
    Then the file "config.yaml" should not exist
    And the exit code should be 0
