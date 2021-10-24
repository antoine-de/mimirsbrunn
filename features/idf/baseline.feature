Feature: Baseline
    Some scenarios for validating in Ile de France

    Background:
        Given admins have been indexed for ile-de-france as idf
        And addresses (bano) have been indexed for 75, 77, 78, 91, 92, 93, 94, 95 into ile-de-france as idf
        And streets have been indexed for ile-de-france as idf
        And stops have been indexed for fr-idf as idf
        And pois have been indexed for fr-idf as idf

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    # These queries are for varying levels of administrative regions (city,
    # department, ...).
    Scenario Outline: Admins exact match
        When the user searches admin datatype for "<query>"
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
        When the user searches addr datatype for "<query>"
        Then he finds "<id>" as the first result

        Examples:
            | query                 | id                         |
            | 20 rue Hector Malot   | addr:2.37715;48.846781:20  |
