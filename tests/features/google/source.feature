# Created by robwilliamson at 1/4/25
Feature: Google Source
  Google data source.

  Scenario: Before Initialisation
    Given a test config
    When I call run
    Then it waits for initialisation
