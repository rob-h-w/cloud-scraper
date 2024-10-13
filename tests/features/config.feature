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
    When I enter "test@test.com"
    When I enter ""
    When I enter ""
    Then the file "config-test.yaml" should exist
    And the file "config-test.yaml" should be a valid config
    And the stdout should have been:
    """Please enter the email you'd like to use as the admin contact when requesting a certificate (you can leave this blank if you don't want to host a secure site using HTTPS):
Please enter the url you'd like to use for serving web traffic (leave blank for http://localhost):
Please enter the folder where site state will be stored (leave blank for .site):
    """
    And the file "config-test.yaml" should contain:
    """email: test@test.com
    """
    And the exit code should be 0

  Scenario: Entering a local path with a nonstandard port generates a file
    Given no file named "config.yaml"
    When I run "cloud_scraper config"
    When I enter ""
    When I enter "http://localhost:12345"
    When I enter ""
    When I enter ""
    Then the file "config.yaml" should exist
    And the file "config.yaml" should be a valid config
    And the stdout should have been:
    """Please enter the email you'd like to use as the admin contact when requesting a certificate (you can leave this blank if you don't want to host a secure site using HTTPS):
Please enter the url you'd like to use for serving web traffic (leave blank for http://localhost):
If you would like to use a different URL visible externally, please provide it here (leave blank if the URL you entered above is visible externally):
Please enter the folder where site state will be stored (leave blank for .site):
    """
    And the file "config.yaml" should contain:
    """domain_config:
  url: http://localhost:12345/
    """
    And the exit code should be 0

  Scenario: Entering a site path generates a file
    Given no file named "config.yaml"
    When I run "cloud_scraper config"
    When I enter ""
    When I enter ""
    When I enter ".my_site"
    Then the file "config.yaml" should exist
    And the file "config.yaml" should be a valid config
    And the stdout should have been:
    """Please enter the email you'd like to use as the admin contact when requesting a certificate (you can leave this blank if you don't want to host a secure site using HTTPS):
Please enter the url you'd like to use for serving web traffic (leave blank for http://localhost):
Please enter the folder where site state will be stored (leave blank for .site):
    """
    And the file "config.yaml" should contain:
    """site_state_folder: .my_site
    """
    And the exit code should be 0

  Scenario: Using a TLS connection without an email address causes an error
    Given no file named "config.yaml"
    When I run "cloud_scraper config"
    When I enter ""
    When I enter "https://my.site"
    When I enter ""
    When I enter ""
    When I enter "1"
    When I enter "1"
    When I enter ""
    Then the file "config.yaml" should not exist
    And the stdout should have been:
    """Please enter the email you'd like to use as the admin contact when requesting a certificate (you can leave this blank if you don't want to host a secure site using HTTPS):
Please enter the url you'd like to use for serving web traffic (leave blank for http://localhost):
If you would like to use a different URL visible externally, please provide it here (leave blank if the URL you entered above is visible externally):
Please enter the email you'd like to use as a contact for the domain (leave blank to finish, an empty list will be replaced with the admin contact email):
Please enter the number of poll attempts to make when retrieving a domain certificate:
Please enter the number of seconds to wait between poll attempts:
Please enter the folder where site state will be stored (leave blank for .site):
    """
    And the stderr should have matched:
    """.*https://my.site/ uses HTTPS, but no email address was provided for certificate requests.*
    """
    And the exit code should not be 0

  Scenario: Entering all data generates a fileServe env debug with empty config and a cli exit override parameter
    Given no file named "config.yaml"
    When I run "cloud_scraper config"
    When I enter "email@test.scenario.domain"
    When I enter "http://test.scenario.domain"
    When I enter "https://external.uri:8080/path"
    When I enter "email-1@domain.owner.contact"
    When I enter "email-2@domain.owner.contact"
    When I enter ""
    When I enter " 3000   "
    When I enter "1"
    When I enter ".my_site_state_folder"
    Then the file "config.yaml" should exist
    And the file "config.yaml" should be a valid config
    And the stdout should have been:
    """Please enter the email you'd like to use as the admin contact when requesting a certificate (you can leave this blank if you don't want to host a secure site using HTTPS):
Please enter the url you'd like to use for serving web traffic (leave blank for http://localhost):
If you would like to use a different URL visible externally, please provide it here (leave blank if the URL you entered above is visible externally):
Please enter the email you'd like to use as a contact for the domain (leave blank to finish, an empty list will be replaced with the admin contact email):
Please enter the email you'd like to use as a contact for the domain (leave blank to finish, an empty list will be replaced with the admin contact email):
Please enter the email you'd like to use as a contact for the domain (leave blank to finish, an empty list will be replaced with the admin contact email):
Please enter the number of poll attempts to make when retrieving a domain certificate:
Please enter the number of seconds to wait between poll attempts:
Please enter the folder where site state will be stored (leave blank for .site):
    """
    And the file "config.yaml" should contain:
    """domain_config:
  external_url: https://external.uri:8080/path
  tls_config:
    builder_contacts:
    - email-1@domain.owner.contact
    - email-2@domain.owner.contact
    poll_attempts: 3000
    poll_interval_seconds: 1
  url: http://test.scenario.domain/
email: email@test.scenario.domain
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
