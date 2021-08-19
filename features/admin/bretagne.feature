Feature: Baseline
    Some scenarios for testing admins in Britany, France.
    Part of the reason for britany is that it does not take too long to index
    into Elasticsearch.

    Background:
        Given osm file has been downloaded for bretagne
        And osm file has been processed by cosmogony for bretagne
        And cosmogony file has been indexed for bretagne

    # With 'Exact Match', we expect the query to be found at the top of the
    # result because the query exactly matches the name / label of the target.
    # These queries are for varying levels of administrative regions (city,
    # department, ...).
    Scenario Outline: Exact Match
        When the user searches for "<query>"
        Then he finds "<id>" as the first result

        Examples:
            | query             | id                        |
            | Côtes-d'Armor     | admin:osm:relation:7398   |
            | Loire-Atlantique  | admin:osm:relation:7432   |
            | Lorient           | admin:osm:relation:30305  |
            | Quimper           | admin:osm:relation:296095 |
            | Saint-Malo        | admin:osm:relation:905534 |

            # These cities have homonyms, we expect the one with the greatest
            # weight.
            | Tréméven          | admin:osm:relation:74058  |
            | Saint-Armel       | admin:osm:relation:145091 |
            | Plouhinec         | admin:osm:relation:122789 |

            # Saint-Malo is the biggest city with "saint" in its name.
            | Saint             | admin:osm:relation:905534 |
