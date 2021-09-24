# Indexing

Indexing is the process by which we take data sources and store them as indexes
in Elasticsearch. This process involves several steps of parsing, validating,
and enriching the data prior to the actual indexing.

## Getting Started

Indexing is done with several binaries that are built within the Mimirsbrunn project. Assuming you
have a rust environment, you can just do the following:

```
git clone git@github.com:CanalTP/mimirsbrunn.git
cd mimirsbrunn
cargo build --release
```

This will create several executable in `./target/release/{osm2mimir, cosmogony2mimir, ...}

## Usage

The following table shows what binary and source of data you need depending on the type of data
you want to index:

<table style="width:92%;">
<colgroup>
<col style="width: 27%" />
<col style="width: 31%" />
<col style="width: 31%" />
</colgroup>
<tbody>
<tr class="odd">
<td>type</td>
<td>binary</td>
<td>data source</td>
</tr>
<tr class="even">
<td>administrative regions (admins)</td>
<td>cosmogony2mimir</td>
<td>cosmogony</td>
</tr>
<tr class="odd">
<td>streets</td>
<td>osm2mimir</td>
<td>OSM</td>
</tr>
<tr class="even">
<td>addresses</td>
<td>bano2mimir / openaddress2mimir</td>
<td>BANO (France)</td>
</tr>
<tr class="odd">
<td>public points of interests (POI)</td>
<td>osm2mimir</td>
<td>OSM</td>
</tr>
<tr class="even">
<td>private points of interests (POI)</td>
<td>poi2mimir</td>
<td></td>
</tr>
</tbody>
</table>

If you want to index all of those data types for a given geographical regions, your process will 
involve the following, which will be described below:

1. Download the datasets
2. Launch an Elasticsearch
3. Launch indexing binaries in a specific order

There is a currently two tools that will help you wrap these steps in an easy and configurable

* [Fafnir](https://github.com/QwantResearch/fafnir)
* import2mimir is a bash script 

### Download datasets

#### cosmogony

cosmogony datasets are produced by a binary cosmogony available
[here](https://github.com/osm-without-borders/cosmogony), which uses an OSM pbf
input.

cosmogony is a rust project, so you need to build it first (these commands are
taken from cosmogony's README, so you may want to see for updates first)

```
curl https://sh.rustup.rs -sSf | sh    # intall rust
apt-get install libgeos-dev            # install GEOS
git clone https://github.com/osm-without-borders/cosmogony.git     # Clone this repo
cd cosmogony;                          # enter the directory
git submodule update --init            # update the git submodules
cargo build --release                  # finally build cosmogony
```

cosmogony run on OSM pbf documents, which you need to download. Once you have a dataset, you can run

```
./target/release/cosmogony -i <path/to/source.osm.pbf> -o <path/to/output.ext>
```

The output extension can be any of {`json`, `jsonl`, `json.gz`, `jsonl.gz`}

#### OSM

You can download OpenStreetMap data for different regions of the world from
[geofabrik](http://download.geofabrik.de/). Be sure to download no more than
you need. Larger files will take longer to index, and will take more memory in
Elasticsearch.

#### BANO


### Launch Elasticsearch

You can either benefits from a full blown elasticsearch deployment, or, for
evaluation purposes, use a docker container. Here is the command that will
create and run such a container:

```
docker run -p 9200:9200 -p 9300:9300 \
  -e "discovery.type=single-node" --name "elasticsearch" \
  -d docker.elastic.co/elasticsearch/elasticsearch:7.13.0
```

### Index Data

We mentioned earlier that your data is parsed, validated, and enriched before
beeing indexed into Elasticsearch. So processing the data may require to
extract previously indexed data from Elasticsearch to add context to a
geospatial place. For example, if we index an address from BANO, we may want to
add a reference to a street and to the administrative region it belongs to.

Here is the order of execution:
1. `cosmogony2mimir`
2. `osm2mimir`
3. `bano2mimir` / `openaddress2mimir`
4. `poi2mimir`

### import2mimir


