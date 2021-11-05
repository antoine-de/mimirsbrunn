+------------------------+---------------------------+---------------------------------------------------------------+---------+
| field                  | type                      | description                                                   | source  |
+========================+===========================+===============================================================+=========+
| approx_coord           | Option<Geometry>          |                                                               | address |
| context                | Option<Context>           |                                                               | address |
| coord                  | Coord                     |                                                               | address |
| country_codes          | Vec<String>               |                                                               | address |
| house_number           | String                    | Identifier in the street                                      | address |
| id                     | String                    | Unique identifier                                             | address |
| label                  | String                    |                                                               | address |
| name                   | String                    |                                                               | address |
| street                 | Street                    | Reference to the street the address belongs to.               | address |
| weight                 | f64                       |                                                               | address |
| zip_codes              | Vec<String>               |                                                               | address |
| administrative_regions | Vec<Arc<Admin>>           | A list of parent administrative regions                       | admin   |
| approx_coord           | Option<Geometry>          | Coordinates of (the center??) of the region, similar to coord | admin   |
| bbox                   | Option<Rect<f64>>         | Bounding Box                                                  | admin   |
| boundary               | Option<MultiPolygon<f64>> | Describes the shape of the admin region                       | admin   |
| codes                  | BTreeMap<String, String>  | Some codes used in OSM, like ISO3166, ref:nuts, wikidata      | admin   |
| context                | Option<Context>           | Used for debugging                                            | admin   |
| coord                  | Coord                     | Coordinates of the region                                     | admin   |
| country_codes          | Vec<String>               | Country Codes                                                 | admin   |
| id                     | String                    | Unique id created by cosmogony                                | admin   |
| insee                  | String                    | A code used to identify regions in France.                    | admin   |
| label                  | String                    | ??                                                            | admin   |
| labels                 | I18nProperties            | ??                                                            | admin   |
| level                  | u32                       | Position of the region in the admin hierarchy                 | admin   |
| name                   | String                    | Name                                                          | admin   |
| names                  | I18nProperties            | Name, but internationalized, eg name:en, name:ru, name:es     | admin   |
| parent_id              | Option<String>            | id of the parent admin region (or none if root)               | admin   |
| weight                 | f64                       | A number associated with the population in that region        | admin   |
| zip_codes              | Vec<String>               | Zip codes (can be more than one)                              | admin   |
| zone_type              | Option<ZoneType>          | Describes the type, eg city, suburb, country,…                | admin   |
| address                | Option<Address>           | Address associated with that POI                              | poi     |
| administrative_regions | Vec<Arc<Admin>>           |                                                               | poi     |
| approx_coord           | Option<Geometry>          |                                                               | poi     |
| context                | Option<Context>           |                                                               | poi     |
| coord                  | Coord                     |                                                               | poi     |
| country_codes          | Vec<String>               |                                                               | poi     |
| id                     | String                    |                                                               | poi     |
| label                  | String                    |                                                               | poi     |
| labels                 | I18nProperties            |                                                               | poi     |
| name                   | String                    |                                                               | poi     |
| names                  | I18nProperties            |                                                               | poi     |
| poi_type               | PoiType                   | id / name references in NTFS                                  | poi     |
| properties             | BTreeMap<String, String>  |                                                               | poi     |
| weight                 | f64                       |                                                               | poi     |
| zip_codes              | Vec<String>               |                                                               | poi     |
| administrative_regions | Vec<Arc<Admin>>           |                                                               | stop    |
| approx_coord           | Option<Geometry>          |                                                               | stop    |
| codes                  | BTreeMap<String, String>  |                                                               | stop    |
| comments               | Vec<Comment>              |                                                               | stop    |
| commercial_modes       | Vec<CommercialMode>       |                                                               | stop    |
| context                | Option<Context>           |                                                               | stop    |
| coord                  | Coord                     |                                                               | stop    |
| country_codes          | Vec<String>               |                                                               | stop    |
| coverages              | Vec<String>               |                                                               | stop    |
| feed_publishers        | Vec<FeedPublisher>        |                                                               | stop    |
| id                     | String                    |                                                               | stop    |
| label                  | String                    |                                                               | stop    |
| lines                  | Vec<Line>                 |                                                               | stop    |
| name                   | String                    |                                                               | stop    |
| physical_modes         | Vec<PhysicalMode>         |                                                               | stop    |
| properties             | BTreeMap<String, String>  |                                                               | stop    |
| timezone               | String                    |                                                               | stop    |
| weight                 | f64                       | The weight depends on the number of lines, and                | stop    |
| zip_codes              | Vec<String>               |                                                               | stop    |
| administrative_regions | Vec<Arc<Admin>>           |                                                               | street  |
| approx_coord           | Option<Geometry>          |                                                               | street  |
| context                | Option<Context>           |                                                               | street  |
| coord                  | Coord                     |                                                               | street  |
| country_codes          | Vec<String>               |                                                               | street  |
| id                     | String                    |                                                               | street  |
| label                  | String                    |                                                               | street  |
| name                   | String                    |                                                               | street  |
| weight                 | f64                       |                                                               | street  |
| zip_codes              | Vec<String>               |                                                               | street  |
