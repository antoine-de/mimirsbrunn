[![travis](https://travis-ci.org/CanalTP/mimirsbrunn.svg?branch=master)](https://travis-ci.org/CanalTP/mimirsbrunn)
[![GitHub license](https://img.shields.io/github/license/CanalTP/mimirsbrunn.svg)](https://github.com/CanalTP/mimirsbrunn/blob/master/LICENSE)
[![GitHub tag](https://img.shields.io/github/tag/CanalTP/mimirsbrunn.svg)](https://github.com/CanalTP/mimirsbrunn/tag)

# Mimirsbrunn

Mimirsbrunn (also called Mimir) is an independent geocoding and reverse-geocoding system written in [Rust](https://www.rust-lang.org/en-US/), and built upon [Elasticsearch](https://www.elastic.co).
It can handle addresses, streets, points-of-interest (POI), administrative regions and public transport stops.

# How to use

## API

Mimirsbrunn exposes a [REST](https://en.wikipedia.org/wiki/Representational_state_transfer) Json api (with [bragi](https://github.com/CanalTP/mimirsbrunn/tree/master/libs/bragi)).

This API provices several services:

### Autocomplete

| feature              | route            | Parameters                                                                                                                                   | response                                                                                                                                                                                                                                                                                      |
| -------------------- | ---------------- | -------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| geocoding            | `/autocomplete`  | TODO (in the meantime, can be seen [here](https://github.com/CanalTP/mimirsbrunn/blob/master/libs/bragi/src/routes/autocomplete.rs#L58-L80)) | The response is formated using [geocodejson](https://github.com/geocoders/geocodejson-spec), the same format as [pelias](https://github.com/pelias/pelias), [photon](https://github.com/komoot/photon) and [addok](https://github.com/addok/addok). TODO: give more details and some examples |
| reverse geocoding    | `/reverse`       | TODO (in the meantime, can be seen [here](https://github.com/CanalTP/mimirsbrunn/blob/master/libs/bragi/src/routes/reverse.rs#L9-L14))       | TODO: give more details and some examples                                                                                                                                                                                                                                                     |
| Detail on one object | `/features/{id}` | TODO (in the meantime, can be seen [here](https://github.com/CanalTP/mimirsbrunn/blob/master/libs/bragi/src/routes/features.rs#L8))          | TODO: give more details and some examples                                                                                                                                                                                                                                                     |

### Monitoring API

| feature            | route      | Parameters |
| ------------------ | ---------- | ---------- |
| staus              | `/status`  | None       |
| Prometheus metrics | `/metrics` | None       |


## handled datasets

Mimirsbrunn relies on geographical datasets to find what users are looking for.
These locations belong to different data types and come from various sources.
To import these locations Mimirsbrunn comes along with the following specific tools:

|       Data Types       |                     Data Sources                      |            Import Tools           |
| :--------------------: | :---------------------------------------------------: | :-------------------------------: |
|       Addresses        | OpenAddresses  or BANO (the french opendata dataset)  | openaddresses2mimir or bano2mimir |
|        Streets         |                     OpenStreetMap                     |             osm2mimir             |
|          POI           |                     OpenStreetMap                     |             osm2mimir             |
| Public Transport Stops | Navitia.io data platform  or any GTFS data repository |     ntfs2mimir or stops2mimir     |
| Administrative Regions |              OpenStreetMap or Cosmogony               |   osm2mimir or cosmogony2mimir    |

To use another datasource you have to write your own data importer.
See for instance [Fafnir](https://github.com/QwantResearch/fafnir), an external component to import POIs from another database.

For more detail about the different ways to import those data sources, check the [components documentation](https://github.com/CanalTP/mimirsbrunn/blob/master/documentation/components.md).

# Install

## with docker

[docker_mimir](https://github.com/QwantResearch/docker_mimir) is a repository with some python script to easily import some data in mimir.

## debian packages

[Kisio Digital](http://www.kisiodigital.com/), the company behind
[navitia](https://www.navitia.io/) has some available debian
8 [packages](https://ci.navitia.io/view/mimir/job/mimirsbrunn_release_package/lastSuccessfulBuild/artifact/mimirsbrunn_1.8.0_amd64.deb).

If you need some packages for a different target, you can use CanalTP's
[script](https://github.com/CanalTP/mimirsbrunn/blob/master/build_packages.sh) or
[cargo-deb](https://github.com/mmstick/cargo-deb)

## manually

### prerequiste
* [install rust](https://rustup.rs/)
* install ES: the supported ES version is 2.x (yes it's old...).
You can install it either directly on you system, or use docker.
For a disposable ES, you can run:

`docker run --name es2 -d -p '9200:9200' elasticsearch:2`

### build

#### Binaries

If you want to build local binaries, use `cargo build --release`. You may need to adjust the
version of the rust compiler prior to executing this command (eg `rustup update`).

#### Docker

If you want to build a docker image, you can use the `docker` command, with the following caveat:
You need to specify the version of debian it will be based on, as well as the version of rust.
You also want to verify that the version of Debian you specify is supported! Check the content of
`docker/Dockerfile_bragi` for that. Your `docker` command will look like:

```
docker build --build-arg DEBIAN_VERSION=buster --build-arg RUST_VERSION=1.47 ...
```

Alternatively, there is a Makefile that can assist you with several tasks. See `make help` for
details. For example `make snapshot` will build an image and push it to a repository. The
repository, as well as the build environments are specified in `deploy.env`.

### test

`cargo test`

Integration tests are spawning one ElasticSearch docker, so you'll need a recent docker version.
Only one docker is spawn, so the ES db is cleaned before each test.

# More documentation

For more precise documentation on use, troubleshooting, development please check the [documentation
directory](docs/readme.md).

# Contributing

The project is [Licensed as AGPL 3](https://github.com/CanalTP/mimirsbrunn/blob/master/LICENSE).

We'll be happy to review all you [pull
requests](https://help.github.com/en/articles/about-pull-requests).

You can also open some issues if you find some bugs, or if you find the geocoder results not to
your liking.

Another way to contribute to this project (and to lots of others geocoders) is to add some geocoder
test on [geocoder-tester](https://github.com/geocoders/geocoder-tester), a great non regression
tool used by many geocoder. It's quite easy, you just add some new test cases with some searches
and the results that you expect.








# Mimirsbrunn

Mimirsbrunn (also called Mimir) is an independent geocoding and reverse-geocoding system written in
[Rust](https://www.rust-lang.org/en-US/), and built upon [Elasticsearch](https://www.elastic.co). It
can handle addresses, streets, points-of-interest (POI), administrative regions and public transport
stops.

## What's a Geocoder ?

Usually [geocoding](https://en.wikipedia.org/wiki/Geocoding) refers to "the process of transforming
a physical address description to a location on the Earth's surface". However Mimir is more a
[geoparser](https://en.wikipedia.org/wiki/Toponym_resolution#Geoparsing) than a geocoder since it
can resolve any ambiguous toponym to its correct location.

In other words, a geocoder reads a description (possibly incomplete) of a location, and returns a
list of candidate locations (latitude / longitude) matching the input. 

Geocoding is traditionally used for autocompleting search fields used in geographic applications.
For example, here is a screenshot of Qwant Maps, where the user enters a search string `20 rue hec
mal`, and mimir returns possible candidates in a dropdown box.

![qwant maps](https://user-images.githubusercontent.com/3987698/56976025-53ed1180-6b72-11e9-9c81-9718e92061ce.png)

## Getting Started

These instructions will give you a copy of the project up and running on your local machine for
development and testing purposes. See deployment for notes on deploying the project on a live
system.

### Prerequisites

Mimirsbrunn is a rust project, so you need to have a rust development environment. Instructions on
how to do that are found on the [rust website](https://www.rust-lang.org/tools/install).

Additionally, mimirsbrunn relies on [cosmogony](https://github.com/osm-without-borders/cosmogony) to
create so called cosmogony files which are used for indexing administrative regions.

For running end to end or unit tests, you need the docker engine available.

### Installing

You can install Mimirsbrunn from debian packages available as build artifacts on the repository
homepage. [FIXME Where are .deb ?]

You can also build Mimirsbrunn manually, as the following instructions explain.

You need to retrieve the project and build it using the rust compiler:

```
git clone https://github.com/CanalTP/mimirsbrunn.git
cd mimirsbrunn
cargo build --release
```

Having built the project, you can now perform a sanity check. This will serve as a test that the
program works, and as a basic example of the two main functionalities: indexing and querying.

So we'll index administrative regions into Elasticsearch, and then search for some of them. We'll
focus on one country, lets say Denmark.

1. Download OSM file.

First we download the OSM file for Denmark, from the
[geofabrik](http://download.geofabrik.de/europe/denmark-latest.osm.pbf) server, and store the file
locally.

2. Generate cosmogony file.

If you haven't installed cosmogony yet, you need to do so now, by following the instructions
[here](https://github.com/osm-without-borders/cosmogony). You can then transform the original
OSM PBF file for Denmark:

```
cargo run --release -- generate -i /path/to/denmark-latest.osm.pbf -o denmark.jsonl.gz
```

3. Start an Elasticsearch docker container.

We'll start an Elasticsearch version 7.13, available on ports 9200 and 9300.

```
docker run --name mimirsbrunn-test -p 9200:9200 -p 9300:9300 -e "discovery.type=single-node" -d docker.elastic.co/elasticsearch/elasticsearch:7.13.0
```

You can check that Elasticsearch has correctly started (maybe wait for about 10/20s for
Elasticsearch to be available):

```
curl 'http://localhost:9200'
{
  "name": "...",
	"cluster_name": "docker-cluster",
	...
	"tagline": "You Know, for Search"
}
```

4. Index cosmogony into Elasticsearch

The result of building the mimirsbrunn project includes several binaries located in
`/target/releases`, one of which is used to index cosmogony files:

cosmogony2mimir uses several configuration files found in the source code, and they work fine by
default. In the following command, we use a setting to make sure that cosmogony2mimir will target
the Elasticsearch container we just started: (See [here](/docs/indexings.md#cosmogony2mimir) for
more details about using cosmogony2mimir)

```
cd mimirsbrunn
./target/release/cosmogony2mimir \
  -c ./config \
	-m testing \
	-s elasticsearch.url='http://localhost:9200' \
	-s langs=['en', 'da'] \
	-i <path/to/denmark.jsonl.gz> \
	run
```

You can follow in the `mimirsbrunn/logs` directory.

5. Check Elasticsearch

The previous step created an index for administrative regions, and so you should be able to query your
Elasticsearch like so.

```
curl 'http://localhost:9200/_cat/indices'
```

6. Start Bragi

7. Query Bragi

## Running the Tests

Since this is a rust project, we are well instrumented to run all sorts of tests:
* style
* lint
* unit tests
* end to end / integration.

You can run them all at once, and this in the way it is carried out in the CI pipeline, with 

```
make check
```

One thing to care about while running tests is that you need to take into account that this project
has rust workspaces, and also uses features for conditional compilation.

See this [page](/docs/testing.md) for a more in depth introduction to testing this project.

### Style

We don't follow any particular style, just the default rust style, and you can check that your
contribution follows the style with:

```
cargo fmt --all -- --check
```

### Lint

Again we follow the linting suggestions given by *clippy*:

```
cargo clippy --all-targets -- --warn clippy::cargo --allow clippy::multiple_crate_versions --deny warnings
```

### Unit Tests

### End to End Tests

## Deployment

Add additional notes to deploy this on a live system

## Usage

Mimirsbrunn functionality is split into two domain:
* Indexing data, that is the process of taking raw geospatial data and storing it into
	Elasticsearch. You will find detailed description of the processes involved in a separate
	[page](docs/indexing.md).
* Querying data, which is finding the relevant data, based on query parameters. We provide
  [Bragi](docs/bragi.md) for that purpose.

## Contributing

You can find more developer oriented documentation [here](/docs/developer.md)

Please read [CONTRIBUTING.md](CONTRIBUTING.md) for details on our code of conduct, and the process
for submitting pull requests to us.

## Who Uses It ?

* [Navitia](https://github.com/CanalTP/navitia)
* [Qwant Maps](https://www.qwant.com/maps)

If you use it too, feel free to open a pull request, we'll be happy to add your project here!

# Alternatives

* [pelias](https://github.com/pelias/pelias)
* [photon](https://github.com/komoot/photon)
* [addok](https://github.com/addok/addok)

TODO: add a bit more detail on all the projects

All those projects use quite the same APIs, and you can compare their results using
[geocoder-tester](https://github.com/geocoders/geocoder-tester).

For a more visual comparison, you can also use [a comparator](https://github.com/CanalTP/autocomplete-comparator).

## Ressources

* [A french presentation of Mimirsbrunn](https://github.com/TeXitoi/pinot2017bano/blob/master/pinot2017bano.pdf)

## Versioning

We use [Semantic Versioning](http://semver.org/) for versioning. For the versions available, see the
[tags on this repository](https://github.com/CanalTP/mimirsbrunn/tags).

## Authors

Mimirsbrunn is a project initially spearheaded by [Guillaume Pinot](texitoi@texitio.eu) and [Antoine
Desbordes]() for [Navitia](http://navitia.io). 

See also the list of [contributors](https://github.com/CanalTP/mimirsbrunn/contributors) who
participated in this project.

## License

This project is licensed under the [AGPLv3](LICENSE.md) GNU Affero General Public License - see the
[LICENSE.md](LICENSE.md) file for details

## Acknowledgments

  - **Billie Thompson** - *Provided README Template* - [PurpleBooth](https://github.com/PurpleBooth)

