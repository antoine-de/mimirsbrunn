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
