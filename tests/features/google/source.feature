# Created by robwilliamson at 1/4/25
Feature: Google Source
  Google data source.

  Scenario: Before Initialisation
    Given a test config
    When I call run
    Then it releases the semaphore
    And it waits for initialisation

  Scenario: After Initialisation
    Given a test config
    When I call run
    When I send_init
    Then it replies to init with ()
