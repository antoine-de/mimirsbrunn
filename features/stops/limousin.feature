Feature: Stops
    Some scenarios for testing stops in Limousin, France.

    Background:
        Given osm file has been downloaded for limousin
        And osm file has been processed by cosmogony for limousin
        And cosmogony file has been indexed for limousin as limousin
        And ntfs file has been indexed for limousin as limousin

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    Scenario Outline: Stops exact match
        When the user searches "stop" for "<query>"
        Then he finds "<id>" as the first result

        Examples:
            | query                  | id               |
	    | charles de gaulle      | stop_area:CDG    |
