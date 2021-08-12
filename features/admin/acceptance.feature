Feature: Baseline
  List of scenarios for geocoding

  @fullSpell @street
  Scenario Outline: Simple street search
    Given admins have been loaded using cosmogony from bretagne
    When the user searches for "<query>"
    Then he finds "<id>" in the first <limit> results.

    Examples:
      | query             | id             | limit       |
      | rue hector malot  | id3234         | 3           |
