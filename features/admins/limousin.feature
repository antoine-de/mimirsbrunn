Feature: Admins
    Some scenarios for testing admins in Limousin, France.
    The reason for picking Limousin is that the size of the OSM file
    is less than 100Mb, which is the upper file size limit for github.

    Background:
        Given admins have been indexed for limousin

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    # These queries are for varying levels of administrative regions (city,
    # department, ...).
    @unittest
    Scenario Outline: Admins exact match
        When the user searches admin datatype for "<query>"
        Then he finds admin "<name>", a "<zone_type>", in the first <limit> results

        Examples:
            | query             | name           | zone_type   | limit  |
            | Creuse            | Creuse         | state       | 1      |
            | Haute-Vienne      | Haute-Vienne   | state       | 1      |
            | Limoges           | Limoges        | city        | 1      |
            | Saint-Junien      | Saint-Junien   | city        | 1      |
