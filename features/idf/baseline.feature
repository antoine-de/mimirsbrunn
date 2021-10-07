Feature: Baseline
    Some scenarios for validating in Ile de France

    Background:
        Given osm file has been downloaded for ile-de-france
        And osm file has been processed by cosmogony for ile-de-france
        And cosmogony file has been indexed for ile-de-france
	And bano files have been downloaded for 92, 75, 94 into ile-de-france
	And bano file has been indexed for ile-de-france

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    # These queries are for varying levels of administrative regions (city,
    # department, ...).
    Scenario Outline: Admins exact match
        When the user searches "admin" for "<query>"
        Then he finds "<id>" as the first result

        Examples:
            | query             | id                         |
	    | paris             | admin:osm:relation:7444    | 
            | ile-de-france     | admin:osm:relation:8649    |
            | saint-denis       | admin:osm:relation:87922   |

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    # These queries are for varying levels of administrative regions (city,
    # department, ...).
    Scenario Outline: Addresses exact match
        When the user searches "addr" for "<query>"
        Then he finds "<id>" as the first result

        Examples:
            | query                 | id                         |
	    | 20 rue Hector Malot   | addr:2.37715;48.846781:20  |
