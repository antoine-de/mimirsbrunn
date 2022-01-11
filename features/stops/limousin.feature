Feature: Stops
    Some scenarios for testing stops in Limousin, France.

    Background:
        Given osm file has been downloaded for limousin
        And osm file has been processed by cosmogony for limousin
        And cosmogony file has been indexed for limousin as limousin
        And ntfs file has been indexed for limousin as limousin

    # /autocomplete endpoint
    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    @unittest
    Scenario Outline: Stops exact match
        When the user searches stop datatype for "<query>"
        Then he finds "<id>" as the first result

        Examples:
            | query                  | id               |
            | charles de gaulle      | stop_area:CDG    |


    # /features endpoint
    # we expect to found the requested id
    @unittest
    Scenario Outline: Stops find by id
        When the user ask for id "<id>" with pt_dataset "<pt_dataset>"
        Then he gets "<id>" as the first result, with name "<stop_name>"

        Examples:
            | id                 | pt_dataset   | stop_name           |
            | stop_area:GDL      | limousin     | Gare de Lyon        |
            | stop_area:CDG      | limousin,idf | Charles de Gaulle   |