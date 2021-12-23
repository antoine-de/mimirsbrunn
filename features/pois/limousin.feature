Feature: Pois
    Some scenarios for testing pois in Limousin, France.
    The reason for picking Limousin is that the size of the OSM file
    is less than 100Mb, which is the upper file size limit for github.

    Background:
        Given admins have been indexed for limousin
        And pois have been indexed for limousin

    # /autocomplete endpoint
    @unittest
    Scenario Outline: Pois exact match
        When the user searches poi datatype for "<query>"
        Then he finds poi "<label>", a "<poi_type>" located near <lat>, <lon> in the first <limit> results

        Examples:
            | query              | label                              | poi_type  | lat     | lon    | limit  |
            | parking saint merd | Parking (Saint-Merd-les-Oussines)  | Parking   | 45.5973 | 2.0703 | 5      |


    # /features endpoint
    @unittest
    Scenario Outline: Pois find by id
        When the user ask for id "<id>" with poi_dataset "<poi_dataset>" and pt_dataset "<pt_dataset>"
        Then he gets "<id>" as the first result, with name "<poi_name>"

        Examples:
            | id                      | pt_dataset   | poi_dataset | poi_name   |
            | poi:osm:node:6923972478 | limousin     | limousin    | Parking    |