# Mímirsbrunn

Mimirsbrunn is an geocoding service build upon [Elasticsearch](https://www.elastic.co).

It is an independent service, but [Navitia](https://github.com/CanalTP/navitia) uses it as it's global geocoding service.

Mimirsbrunn is composed of several [parts](#components), some managing the data import in Elasticsearch, and a web service wrapping Elasticsearch responses to return formated responses (we use [geocodejson](https://github.com/geocoders/geocodejson-spec) as the responses format)

## build

### requirements

To build, you must first install rust:

```shell
curl https://sh.rustup.rs -sSf | sh
```

### build
and then build Mimirsbrunn:

```shell
cargo build --release
```

To use the Mimirsbrunn components you will need an elasticsearch database.

The elasticsearch version need to be >= 2.0

### test

To test simply launch:

```shell
cargo test
```

Integration tests are spawning one ElasticSearch docker, so you'll need a recent docker version. Only one docker is spawn, so ES base has to be cleaned before each test.

To write a new test:

- write your test in a separate file in tests/
- add a call to your test in tests/tests.rs::test_all()
- pass a new ElasticSearchWrapper to your test method to get the right connection string for ES base
- the creation of this ElasticSearchWrapper automatically cleans ES base (you can also refresh ES base, clean up during tests, etc.)

## Architecture

## Indexes architecture

Data are imported in multiple indexes with this structure:
```
munin -> addr_dataset1 -> addr_dataset1_20160101T123200
     |-> addr_dataset2 -> addr_dataset2_20160101T123200
     |-> admin_dataset1 -> admin_dataset1_20160101T123200
     |-> street_dataset1 -> street_dataset1_20160101T123200
```

Munin is the root index, it's an alias used by the frontend (bragi), it pointing to an index for each dataset/document type.
So if we have address data for France and Belgium we will have two indexes: "addr_fr" and "addr_be". These are also aliases, they point to a dated index, this way we can import data in another index without impacting anyone, then switch the alias to point to the new data.

This will give us the ability to only a part of the world without any downtime.

During an update the indexes will be (for the previous example say we update addr_dataset1):

During the data update:
```
munin -> addr_dataset1 -> addr_dataset1_20160101T123200
     |-> addr_dataset2 -> addr_dataset2_20160101T123200
     |-> admin_dataset1 -> admin_dataset1_20160101T123200
     |-> street_dataset1 -> street_dataset1_20160101T123200

addr_dataset2_20160201T123200
```

and when the loading is finished
```
munin -> addr_dataset1
                      |-> addr_dataset1_20160201T123200
     |-> addr_dataset2 -> addr_dataset2_20160101T123200
     |-> admin_dataset1 -> admin_dataset1_20160101T123200
     |-> street_dataset1 -> street_dataset1_20160101T123200


```


There is one major drawback: dataset aren't hermetic since we import multiple OSM files, the area near the border will be in multiple dataset, for now we accept these duplicate. We will be able to filter with shape at import time and/or remove them in bragi.

## <a name=components> components

All Mimirsbrunn's components implement the `--help` (or `-h`) argument to explain it's use

There are several components in Mimirsbrunn:

### osm2mimir

This component imports openstreetmap data into Mimir.

You can get openstreetmap data from <http://download.geofabrik.de/>

eg:

```shell
curl -O http://download.geofabrik.de/europe/france-latest.osm.pbf
```

To import all those data into Mimir, you only have to do:

```shell
./target/release/osm2mimir --input=france-latest.osm.pbf --level=8 --level=9 --import-way --import-admin --import-poi --dataset=france --connection-string=http://localhost:9200
```

level: administrative levels in openstreetmap

### bano2mimir

This component imports bano's data into Mimir.
It is recommanded to run bano integration after osm integration so that addresses are attached to admins.

You can get bano's data from <http://bano.openstreetmap.fr/data/>

eg:

```shell
curl -O http://bano.openstreetmap.fr/data/full.csv.gz
gunzip full.csv.gz
```

To import all those data into Mimir, you only have to do:

```shell
./target/release/bano2mimir -i full.csv --dataset=france --connection-string=http://localhost:9200/
```

The `--connection-string` argument refers to the ElasticSearch url


### stops2mimir

This component imports stops into Mimir.
It is recommanded to run stops integration after osm integration so that stops are attached to admins.

To import all those data into Mimir, you only have to do:

```shell
./target/release/stops2mimir -i stops.txt --dataset=idf --connection-string=http://localhost:9200/
```

The `--connection-string` argument refers to the ElasticSearch url

The stops input file needs to match the NTFS specification (https://github.com/CanalTP/navitia/blob/dev/documentation/ntfs/ntfs_0.6.md)

### Bragi

Bragi is the webservice build around ElasticSearch.
It has been done to hide the ElasticSearch complexity and to return consistent formated response.

Its responses format follow the [geocodejson-spec](https://github.com/geocoders/geocodejson-spec).
It's a format used by other geocoding API (https://github.com/addok/addok or https://github.com/komoot/photon).

To run Bragi:

```shell
./target/release/bragi --connection-string=http://localhost:9200/munin
```

you then can call the API (the default Bragi's listening port is 4000):
```
curl "http://localhost:4000/autocomplete?q=rue+hector+malot"
```
