+-------------+-------------------+--------------------------------------------------------+-----------------------------+
| name        | type              | description                                            | example                     |
+=============+===================+========================================================+=============================+
| q           | string            | query string                                           | `q=lond`                    |
+-------------+-------------------+--------------------------------------------------------+-----------------------------+
| lat         | double (optional) | latitude. Used to boost                                | `lat=45.3456`               |
|             |                   | results in the vicinity                                |                             |
+-------------+-------------------+--------------------------------------------------------+-----------------------------+
| lon         | double (optional) | longitude. Note that if you specify                    |                             |
|             |                   | lat or lon, you must specify the converse.             | `lon=2.4554`                |
+-------------+-------------------+--------------------------------------------------------+-----------------------------+
| datasets    | list of strings   | restrics the search to                                 | `datatasets[]=fr&`          |
|             | (optional)        | the given datasets.                                    | `datasets[]=be`             |
|             |                   |                                                        |                             |
|             |                   | Valid datasets values are specified                    |                             |
|             |                   | at index time                                          |                             |
|             |                   |                                                        |                             |
|             |                   | See [dataset](/docs/concepts.md) for                   |                             |
|             |                   | an explanation of datasets.                            |                             |
+-------------+-------------------+--------------------------------------------------------+-----------------------------+
| type        | list of strings   | restrics the search to the given place                 | `type[]=streets&`           |
|             | (optional)        | types.                                                 | `type[]=zone`               |
|             |                   |                                                        |                             |
|             |                   | Possible values are:                                   |                             |
|             |                   | * house,                                               |                             |
|             |                   | * poi,                                                 |                             |
|             |                   | * public_transport:stop_area,                          |                             |
|             |                   | * street,                                              |                             |
|             |                   | * zone                                                 |                             |
|             |                   |                                                        |                             |
|             |                   | 1. If no type is given, all types are searched.        |                             |
|             |                   | 2. This type parameter is featured in the response.    |                             |
|             |                   | 3. Some types require a *sub type*, eg poi => poi_type |                             |
+-------------+-------------------+--------------------------------------------------------+-----------------------------+
| zone_type   | list of strings   | restrics the search to                                 | `zone_type[]=city&`         |
|             | (optional)        | the given zone types. (1)                              | `zone_type[]=city_district` |
+-------------+-------------------+--------------------------------------------------------+-----------------------------+
| shape_scope | list of strings   | restrics the shape filter to the types                 | `shape_scope[]=street&`     |
|             |                   | listed in shape_scope.                                 | `shape_scope[]=zone`        |
+-------------+-------------------+--------------------------------------------------------+-----------------------------+
