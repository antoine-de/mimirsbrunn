Feature: Admins
    Some scenarios for testing addresses in Limousin, France.

    Background:
        Given osm file has been downloaded for limousin
        And osm file has been processed by cosmogony for limousin
        And cosmogony file has been indexed for limousin
        And bano file has been indexed for limousin

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    Scenario Outline: Exact Match
        When the user searches for "<query>"
        Then he finds "<id>" as the first result

        Examples:
            | query                      | id                         |
            | 14 Place Allègre, Allassac | addr:1.475761;45.257879:14 |
