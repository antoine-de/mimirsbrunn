# mimirsbrunn

mimir data import

## build

To build, you must first install rust:
```shell
curl -s https://static.rust-lang.org/rustup.sh | sudo sh
```
and then build munin:
```shell
cargo build --release
```

To use the mimirsbrunn components you will need an elasticsearch database.

The elasticsearch version need to be > 1.4

## components
There are several components in mimirbrunn:

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
./target/release/bano2mimir bano-data*/bano-*.csv
```
