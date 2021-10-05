Feature: Admins
    Some scenarios for testing admins in Limousin, France.
    The reason for picking Limousin is that the size of the OSM file
    is less than 100Mb, which is the upper file size limit for github.

    Background:
        Given osm file has been downloaded for limousin
        And osm file has been processed by cosmogony for limousin
        And cosmogony file has been indexed for limousin

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    # These queries are for varying levels of administrative regions (city,
    # department, ...).
    Scenario Outline: Admins exact match
        When the user searches for "<query>"
        Then he finds "<id>" as the first result

        Examples:
            | query             | id                        |
            | Creuse            | admin:osm:relation:7459   |
            | Haute-Vienne      | admin:osm:relation:7418   |
            | Limoges           | admin:osm:relation:114172 |
            | Saint-Junien      | admin:osm:relation:116547 |
