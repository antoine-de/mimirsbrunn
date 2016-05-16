# Mímirsbrunn

mimir data import

## build

To build, you must first install rust:
```shell
curl -sSf https://static.rust-lang.org/rustup.sh | sh
```
and then build Mimirsbrunn:
```shell
cargo build --release
```

To use the Mimirsbrunn components you will need an elasticsearch database.

The elasticsearch version need to be >= 2.0

## test

To test simply launch:
```shell
cargo test
```
Integration tests are spawning one ElasticSearch docker, so you'll need a recent docker version.
Only one docker is spawn, so ES base has to be cleaned before each test.

To write a new test:
* write your test in a separate file in tests/
* add a call to your test in tests/tests.rs::test_all()
* pass a new ElasticSearchWrapper to your test method to get the right connection string for ES base
* the creation of this ElasticSearchWrapper automatically cleans ES base
  (you can also refresh ES base, clean up during tests, etc.)

## Indexes architecture
Data are imported in multiple indexes with this structure:

 munin -> addr -> addr_dataset1 -> addr_dataset1_20150101T123200
                -> addr_dataset2 -> addr_dataset2_20150101T123200
                                    addr_dataset2_20150104T123200
        -> admin -> admin_dataset1 -> admin_dataset1_20150101T123200
        -> street -> street_dataset1 -> street_dataset1_20150101T123200

Munin is the root index, it's an alias used by the frontend (skojig), it pointing to an index for each type.
Each type index is also a alias to an index by dataset, so if we have address data for France and Belgium
we will have two indexes: "addr_fr" and "addr_be". These are also aliases, they point to a dated index,
this way we can import data in another index without impacting anyone, then switch the alias to point to the new data.

This will give us the ability to only a part of the world without any downtime.

There is one major drawback: dataset aren't hermetic since we import multiple OSM files, the area near the border
will be in multiple dataset, for now we accept these duplicate.
We will be able to filter with shape at import time and/or remove them in skojig.

## components
There are several components in Mimirsbrunn:

### bano2mimir

This component import bano's data into Mimir.

You can get bano's data from http://bano.openstreetmap.fr/data/

eg:

```shell
curl -O http://bano.openstreetmap.fr/data/old_2014/BANO-France-20140901-csv.zip
unzip BANO-France-20140901-csv.zip
```

To import all those data into Mimir, you only have to do:

```shell
./target/release/bano2mimir -i bano-data*/
```

### osm2mimir

This component import openstreetmap data into Mimir.

You can get openstreetmap data from http://download.geofabrik.de/

eg:

```shell
curl -O http://download.geofabrik.de/europe/france-latest.osm.pbf
```

To import all those data into Mimir, you only have to do:

```shell
./target/release/osm2mimir --input=france-latest.osm.pbf --level=8 --level=7 --connection-string=http://localhost:9200/munin
```

level: administrative levels in openstreetmap

For more information:

```
./target/release/osm2mimir -h
```
