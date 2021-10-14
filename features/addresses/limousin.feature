Feature: Addresses
    Some scenarios for testing addresses in Limousin, France.

    Background:
        Given osm file has been downloaded for limousin
        And osm file has been processed by cosmogony for limousin
        And cosmogony file has been indexed for limousin as limousin
        And bano file has been indexed for limousin as limousin

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    Scenario Outline: Addresses exact match
        When the user searches "addr" for "<query>"
        Then he finds "<id>" as the first result

        Examples:
            | query                           | id                            |
            | 14 Place Allègre, Allassac      | addr:1.475761;45.257879:14    |
            | Rue du Puy Grasset 1470         | addr:1.938496;45.093038:1470  |
            | 32BIS Avenue du Limousin 19230  | addr:1.385946;45.399633:32BIS |

    # When using aliases, we should still fetch the query at the top of the
    # result.
    Scenario Outline: Addresses with aliases
        When the user searches "addr" for "<query>"
        Then he finds "<id>" as the first result

        Examples:
            | query                      | id                            |
            | 14 p Allègre, Allassac     | addr:1.475761;45.257879:14    |
            | 1470 r du Puy Grasset      | addr:1.938496;45.093038:1470  |
            | 32BIS av du Limousin 19230 | addr:1.385946;45.399633:32BIS |
            | 2 rte du chastang          | addr:1.944186;45.092028:2     |
            | rle bridaine 1042          | addr:1.936327;45.091183:1042  |
