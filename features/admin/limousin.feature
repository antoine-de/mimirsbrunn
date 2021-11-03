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
        When the user searches admin datatype for "<query>"
        Then he finds admin "<name>", a "<zone type>", in the first <limit> results

        Examples:
            | query             | name           | zone type   | limit  |
            | Creuse            | creuse         | state       | 1      |
            | Haute-Vienne      | haute-vienne   | state       | 1      |
            | Limoges           | limoges        | city        | 1      |
            | Saint-Junien      | saint-junien   | city        | 1      |
