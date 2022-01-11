Developer Documentation
=======================

  * [Design](#design)
    * [Software Architecture](#software-architecture)
  * [Contributing](#contributing)
  * [Dependencies](#dependencies)
    * [Crates](#crates)
  * [Development Process](#development-process)
  * [Testing](#testing)
  * [Tools](#tools)
     * [import2mimir](#import2mimir)

# Design

## Software Architecture

[Introduction](./architecture.md)

[Bragi](./bragi-architecture.md)

# Contributing

# Dependencies

## Crates

This is a list of (some of) the main crates the projects depends on:

<table>
<colgroup>
<col style="width: 20%" />
<col style="width: 19%" />
<col style="width: 41%" />
<col style="width: 18%" />
</colgroup>
<thead>
<tr class="header">
<th>Domain</th>
<th>Crate</th>
<th>Motivation</th>
<th>Alternatives</th>
</tr>
</thead>
<tbody>
<tr class="odd">
<td>logging</td>
<td>tracing</td>
<td><ul>
<li>Same team as tokio, warp, …</li>
<li>Support opentelemetry</li>
<li>Support tracing, logs</li>
</ul></td>
<td></td>
</tr>
<tr class="even">
<td>error handling</td>
<td>snafu</td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>web framework</td>
<td>warp</td>
<td></td>
<td></td>
</tr>
<tr class="even">
<td>commandline</td>
<td>clap</td>
<td></td>
<td></td>
</tr>
<tr class="odd">
<td>elasticsearch</td>
<td>elasticsearch</td>
<td></td>
<td></td>
</tr>
</tbody>
</table>

# Development Process

# Testing

You will find information about tests in general [here](/docs/process/testing.md).

This section is meant for developing your own tests.

 
# Tools

## import2mimir

import2mimir.sh is a bash script used to easily create and manipulate a test environment. It
contains functions to
* download dataset from remote sources.
* create and initialize a docker-based elasticsearch environment.
* index admins, streets, addresses, stops, pois.

import2mimir2.sh is configured by a small rc file. Here is an annotated example:

```
# The directory in which data is stored
# DATA_DIR="data"

# Elasticsearch host. If you use docker, it should probably stay at localhost.
ES_HOST="localhost"

# The port number for elasticsearch host (9200 + offset, 9300 + offset)
ES_PORT_OFFSET=3

# The name of the dataset
ES_DATASET="idf"

# The name of the image
ES_IMAGE="docker.elastic.co/elasticsearch/elasticsearch:7.13.0"

# The name of the container
ES_NAME="elasticsearch"
```

This first part of the configuration says that we will start an elasticsearch based on the
`${ES_IMAGE}` image. It will publish ports 9203 and 9303. The name of this docker container will be
`${ES_NAME}` (elasticsearch).

The next part is to configure what data is downloaded and indexed.

```
# The departement (French administrative regions) for BANO
# Base d'Adresses Nationale Ouverte data.gouv.fr
BANO_REGION="75 77 78 92 93 94 95"

# NTFS Region to download
NTFS_REGION="fr-idf"

# OSM Region
OSM_REGION="ile-de-france"

# Base directory for cosmogony
COSMO_DIR="../cosmogony"
```

The script takes a configuration file (like the one we just described), and displays its progress on
stdout, as well as record a log file.

```
cd mimirsbrunn
./script/import2mimir.sh -c ./scripts/idf.rc
```

**WARNING** This script will destroy the docker container named `${ES_NAME}` prior to recreating it.
Make sure this is not a container you want to keep.

## autocomplete
