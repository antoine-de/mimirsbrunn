Feature: Baseline
  List of scenarios for geocoding

  Scenario Outline: Simple admin search
		# Given osm file has been downloaded for bretagne
		# Given osm file has been processed by cosmogony for bretagne
		# Given cosmogony file has been indexed for bretagne
    When the user searches for "<query>"
    Then he finds "<id>" in the first <limit> results.

    Examples:
      | query             | id             | limit       |
      | quimper           | id3234         | 3           |
