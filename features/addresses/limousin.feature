Feature: Addresses
    Some scenarios for testing addresses in Limousin, France.

    Background:
        Given admins have been indexed for limousin
        And bano file has been indexed for limousin

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    @unittest
    Scenario Outline: Addresses exact match
        When the user searches addr datatype for "<query>"
        Then he finds address "<house_num>", "<street>", "<city>", and "<postcode>" in the first "<limit>" results

        Examples:
            | query                           | house_num     | street             | city                   | postcode  | limit |
            | 14 Place Allègre, Allassac      | 14            | Place Allègre      | Allassac               | 19240     | 1     |
            | Rue du Puy Grasset 1470         | 1470          | Rue du Puy Grasset | Argentat-sur-Dordogne  | 19400     | 1     |
            | 32BIS Avenue du Limousin 19230  | 32Bis         | Avenue du Limousin | Arnac-Pompadour        | 19230     | 1     |

    # When using aliases, we should still fetch the query at the top of the
    # result.
    @unittest
    Scenario Outline: Addresses with aliases
        When the user searches addr datatype for "<query>"
        Then he finds address "<house_num>", "<street>", "<city>", and "<postcode>" in the first "<limit>" results

        Examples:
            | query                      | house_num     | street             | city                     | postcode  | limit |
            | 14 p Allègre, Allassac     | 14            | Place Allègre      | Allassac                 | 19240     | 1     |
            | 1470 r du Puy Grasset      | 1470          | Rue du Puy Grasset | Argentat-sur-Dordogne    | 19400     | 1     |
            | 32BIS av du Limousin 19230 | 32Bis         | Avenue du Limousin | Arnac-Pompadour          | 19230     | 1     |
            | 2 rte du chastang          | 2             | Route du Chastang  | Argentat-sur-Dordogne    | 19400     | 1     |
            | 1042 rle bridaine          | 1042          | Ruelle Bridaine    | Argentat-sur-Dordogne    | 19400     | 1     |
