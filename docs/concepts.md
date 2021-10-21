*Alphabetical Order*

# Dataset

Dataset are a way to partition the data stored in Elasticsearch

(TODO)

# Poi and Poi Type

# Type

Such a loaded word in computer...

For bragi, the *type* refers to the type of places the user can query. This is
the list (exhaustive) of valid types:

* **house**: This is an address, that is something like a street number and a street name.
    Use `type[]=house` for query parameter.
* **poi**: A point of interest. Use `type[]=poi`.
* **stop_area**: A public transportation stop, like a bus stop, a train station, ...
    Use `type[]=public_transportation:stop_area`.
* **street**: A public way. Note that note all public ways may be indexed (TODO).
    Use `type[]=street`
* **zone**: A region, or more precisely an administrative region. Sometimes refered to as *admin*.

For the autocomplete REST endpoint, the user can specify the type of place he is interested in:

`[...]/autocomplete?q=chatelet&type[]=public_transport:stop_area`

will enable the user to only search for stops that are related to `chatelet`. 

Related to [zone and zone types](#zone_and_zone_types) and [poi and poi_types](#poi_and_poi_types)

# Zone and Zone Types

TODO Check validity

Zone and Zone Types are concepts inherited from *cosmogony*. cosmogony is the
tool used to extract a hierarchy of administrative regions from OSM files. Zone
is just another word for administrative region. And each zone has a zone type.
which must be one of
* suburb,
* city_district,
* city,
* state_district,
* state,
* country_region,
* country,
* non_administrative,

Note that if you specify `type[]=zone`, then you must specify also the `zone_type[]` list.
