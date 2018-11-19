[![travis](https://travis-ci.org/CanalTP/mimirsbrunn.svg?branch=master)](https://travis-ci.org/CanalTP/mimirsbrunn)
[![GitHub license](https://img.shields.io/github/license/CanalTP/mimirsbrunn.svg)](https://github.com/CanalTP/mimirsbrunn/blob/master/LICENSE)
[![GitHub tag](https://img.shields.io/github/tag/CanalTP/mimirsbrunn.svg)](https://github.com/CanalTP/mimirsbrunn/tag)

# Mímirsbrunn

Mimirsbrunn is an independent geocoding and reverse-geocoding system written in [Rust](https://www.rust-lang.org/en-US/) and built upon [Elasticsearch](https://www.elastic.co).
It can handle addresses, streets, points-of-interest (POI), administrative regions or public transport stops.
In particular [Navitia](https://github.com/CanalTP/navitia) uses it as its global geocoding service.

## Getting Started

Mimirsbrunn is composed of several [parts](#components): some of them manage the data import in Elasticsearch while a web service ([bragi](#bragi)) wraps Elasticsearch interactions in order to return formated responses (using [geocodejson](https://github.com/geocoders/geocodejson-spec) as the responses format)

### Install

- To use the Mimirsbrunn components you need an Elasticsearch database (Elasticsearch version needs to be 2.x).
- To build you must first install rust following [these instructions](https://www.rust-lang.org/en-US/install.html).
- Then to build Mimirsbrunn:
```shell
cargo build --release
```

### Data Input

Mimirsbrunn relies on geographical datasets to find what users are looking for.
These locations belong to different data types and come from various sources.
To import these locations Mimirsbrunn comes along with the following specific tools:

Data Types | Data Sources | [Import Tools](#components)
:---: | :---: | :---:
Addresses | OpenAddresses  or BANO (the french opendata dataset) | openaddresses2mimir or bano2mimir
Streets | OpenStreetMap | osm2mimir
POI | OpenStreetMap | osm2mimir
Public Transport Stops | Navitia.io data platform  or any GTFS data repository | ntfs2mimir or stops2mimir
Administrative Regions | OpenStreetMap or Cosmogony | osm2mimir or cosmogony2mimir

To use another datasource you have to write your own data importer.
See for instance [Fafnir](https://github.com/QwantResearch/fafnir), an external component to import POIs from another database.

## <a name=components> Components: Import Tools & Web Service </a>

There are several components in Mimirsbrunn. Most of them are dedicated to the import of data while other are web services ([bragi](#bragi)) to wrap Elasticsearch interactions.
All the Mimirsbrunn's components described below implement the `--help` (or `-h`) argument to explain their use.

### Import Tools

Before using [Bragi](#bragi), you have to import data into Elasticsearch.
The default and easiest way to import data is to use the [docker_mimir](https://github.com/QwantResearch/docker_mimir) tool.
However the following import tools are still possible.

#### osm2mimir

- This tool imports OpenStreetMap data into Mimir. You can get OpenStreetMap data from [Geofabrik](http://download.geofabrik.de/), for instance:
```shell
curl -O http://download.geofabrik.de/europe/france-latest.osm.pbf
```
- Then to import all those data into Mimir, you only have to do:
```shell
cargo run --release --osm2mimir --input=france-latest.osm.pbf --level=8 --level=9 --import-way --import-admin --import-poi --dataset=france --connection-string=http://localhost:9200
```
- The `level` parameter refers to [administrative levels](https://wiki.openstreetmap.org/wiki/Tag:boundary%3Dadministrative) in OpenStreetMap and is used to control which `Admin` to import.

#### bano2mimir

- This tool imports bano's data into Mimir. It is recommended to run bano integration **after** OSM or [Cosmogony](https://github.com/osm-without-borders/cosmogony) integration in order to attach addresses to admins. You can get bano's data from [OpenStreetMap](http://bano.openstreetmap.fr/data/), for instance:
```shell
curl -O http://bano.openstreetmap.fr/data/full.csv.gz
gunzip full.csv.gz
```
- To import all those data into Mimir, you only have to do:
```shell
cargo run --release --bano2mimir -i full.csv --dataset=france --connection-string=http://localhost:9200/
```
- The `--connection-string` argument refers to the ElasticSearch url.

#### ntfs2mimir

- This tool imports data from the ntfs files into Mimir. It is recommended to run ntfs integration **after** OSM or [Cosmogony](https://github.com/osm-without-borders/cosmogony) integration so that stops are attached to admins. You can get these data from [Navitia](https://navitia.opendatasoft.com/explore).

- To import all those data into Mimir, you only have to do:
```shell
cargo run --release --ntfs2mimir -i <path_to_folder_with_ntfs_file> --dataset=idf --connection-string=http://localhost:9200/
```

- The `--connection-string` argument refers to the ElasticSearch url

- The ntfs input file needs to match the [NTFS specification](https://github.com/CanalTP/navitia/blob/dev/documentation/ntfs/ntfs_0.6.md).

#### stops2mimir

- This import tool is still available but is now deprecated because ntfs2mimir imports already stops.

### <a name=bragi> Web Service: Bragi </a>

Bragi is the webservice built around ElasticSearch.
Its purpose is to hide the ElasticSearch complexity and to return consistent formated responses.
Its responses format follow the [geocodejson-spec](https://github.com/geocoders/geocodejson-spec).
This is a format used by other geocoding API such as [Addok](https://github.com/addok/addok) or [Photon](https://github.com/komoot/photon).

- To run Bragi:
```shell
cargo run --release --bragi --connection-string=http://localhost:9200/munin
```

- Then you can call the API (the default Bragi's listening port is 4000):
```
curl "http://localhost:4000/autocomplete?q=rue+hector+malot"
```

## Contribute

### Integration tests

To test, you need to manually build mimir and then simply launch:

```shell
cargo test
```

Integration tests are spawning one ElasticSearch docker, so you'll need a recent docker version. Only one docker is spawn, so ES base has to be cleaned before each test.

To write a new test:

- write your test in a separate file in tests/
- add a call to your test in tests/tests.rs::test_all()
- pass a new ElasticSearchWrapper to your test method to get the right connection string for ES base
- the creation of this ElasticSearchWrapper automatically cleans ES base (you can also refresh ES base, clean up during tests, etc.)

### Geocoding tests

We use [geocoder-tester](https://github.com/geocoders/geocoder-tester) to run real search queries and check the output against expected to prevent regressions.

Feel free to add some tests cases here.

When a new Pull Request is submitted, it will be manually tested using [this repo](https://gitlab.com/QwantResearch/mimir-geocoder-tester/) that loads a bunch of data into the geocoder, runs geocoder-tester and then add the results as a comment in the PR.


## Troubleshooting

### Multiple streets with the same name and admin in OSM

Some almost identical streets are difficult to distinguish in OSM, as
they are in the same admin and have the same name, but still
they are distinct ones.  
Ex: Rue Jean Jaurès in Lille (France)

osm2mimir merges ways with same name and admin by default.

It can also distinguish them if they are part of different Relation:associatedStreet, so
it's the way to go if one wants to have distinct entries: https://wiki.openstreetmap.org/wiki/Relation:associatedStreet
