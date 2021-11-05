Feature: Pois
    Some scenarios for testing pois in Limousin, France.
    The reason for picking Limousin is that the size of the OSM file
    is less than 100Mb, which is the upper file size limit for github.

    Background:
        Given admins have been indexed for limousin
        And pois have been indexed for limousin

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    # These queries are for varying levels of administrative regions (city,
    # department, ...).
    Scenario Outline: Pois exact match
        When the user searches poi datatype for "<query>"
        Then he finds poi "<label>", a "<poi type>", located near <lat>/<lon>, in the first <limit> results

        Examples:
            | query              | label                              | poi type  | lat     | lon    | limit  |
            | parking saint merd | Parking (Saint-Merd-les-Oussines)  | Parking   | 45.5973 | 2.0703 | 5      |
