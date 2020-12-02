---
title: osm2mimir
updatedAt: 2020-11-18T11:33:34+01:00
abstract: notes
image:
  name: desert
  attribution:
    author:
      name: Foo
      link: Bar
    resource:
      link: Baz
genre: tutorial
author:
  fullname: Matthieu Paindavoine
  resource: FooBar
tags: [osm, pbf, osm2mimir, mimirsbrunn]
---

osm2mimir is a rust binary used to import data in OSM
[PBF](https://wiki.openstreetmap.org/wiki/PBF_Format) format into Elasticsearch, suitable for
use by bragi.

osm2mimir is part of the [mimirsbrunn](https://github.com/CanalTP/mimirsbrunn) project, which is
a geocoding system written in Rust, resting on Elasticsearch for storage and indexing.

## Building

If you want to build osm2mimir, you need to setup a rust environment. See the Rust
[homepage](https://www.rust-lang.org/) for details.

You also need to have an Elasticsearch running, which is easy to setup with docker. Unfortunately,
we're currently stuck with an old version of Elasticsearch, but this should change soon.

```
docker run --name es2 -d -p '9200:9200' elasticsearch:2
```

You can then build and run some tests:

```
cargo build --release
cargo test --release
```

## Usage

To run osm2mimir, you may need to adjust the default behavior by adding your own customized
settings. osm2mimir is configured using a layered approach: We use a default configuration, which
can be overwritten by custom settings, and command line arguments. You can have a look at the
default configuration that comes with the code, in `config/osm2mimir-default.toml`.

Typically, you would use a command line like so:

```
osm2mimir
  --config-dir config
  --settings prod
  --import-poi
  --import-way
  --input [file.osm.pbf]
```

This command will read the default configuration in `config/osm2mimir-default.toml`, then merge
specific production settings from `config/prod.toml`. It will import POIs and streets from the file
`file.osm.pbf`

## Configuration

Provide a detailed list of configuration options

## Deployment

Talk about docker ...

