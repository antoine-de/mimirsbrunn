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
For more information:

```
./target/release/osm2mimir -h
```
