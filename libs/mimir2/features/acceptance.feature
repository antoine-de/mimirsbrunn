Feature: Acceptance tests

  Scenario: Baseline scenario
    Given I have generated an index
    When I list all the documents in the index
    Then I find the original list
