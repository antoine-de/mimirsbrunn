Gherkin
=======

  * [Backgrounds](#backgrounds)
    * [Downloading Files](#downloading-files)
    * [Processing Files](#processing-files)
    * [Indexing Files](#indexing-files)
  * [Scenarios](#scenarios)

This document describes the syntax used to specify end to end tests. It is based
[gherkin](https://cucumber.io/docs/gherkin/reference/).

# Backgrounds

A background section is used to specify what data are indexed in a test environment. It is a
preliminary step for the execution of multiple scenarios in that environment. Here is an example:

```gherkin
Background:
    Given osm file has been downloaded for limousin
    And osm file has been processed by cosmogony for limousin
    And cosmogony file has been indexed for limousin as limousin
    And ntfs file has been indexed for limousin as limousin
```

## Downloading files

Some tests requires files to be downloaded because they are too large to be stored along with the
source code.

### OSM

`osm file has been downloaded for <region>`.

Download the file corresponding to the French region from geofabrik server. You can find the list of
regions [here](http://download.geofabrik.de/europe/france.html). The downloaded file is stored in
`tests/fixtures/osm/<region>/<region>-latest.osm.pbf` (according to geofabrik naming conventions).
If a file with that same name already exists, then we skip the download.

### BANO

`bano files has been downloaded for <departments> into <region>`.

Download the files corresponding to the French departments from BANO. The departments is a list of
comma separated numbers each corresponding to a department. The downloaded files are concatenated
into a single BANO file `tests/fixtures/bano/<region>/<region>.csv`.

If a file with that same name already exists, then we skip the download.

### NTFS

`ntfs file has been downloaded for <region>`.

Download the file corresponding to the French region from
[Navitia](https://navitia.opendatasoft.com/) into a directory `tests/fixtures/ntfs/<region>/`. It
also unzips the file in that folder.

If a file with that same name already exists, then we skip the download.

## Processing Files

### Cosmogony

`osm file has been processed by cosmogony for <region>`.

Calls cosmogony on the OSM file found in `tests/fixtures/osm/<region>/<region>-latest.osm.pbf` and
stores the result in `tests/fixtures/cosmogony/<region>/<region>.jsonl.gz`.

If a cosmogony file with that same name already exists, then we skip the download.

## Indexing Files

**Warning** Because this is a test environment, we use the **testing** configuration for
Elasticsearch, specified in
[`config/elasticsearch/testing.toml`](/config/elasticsearch/testing.toml). Make sure this file
points to the correct backend before starting the test.

**Warning** If you want to index multiple types of data in a background, make sure the indices are
built in a certain order.

### Admins

`cosmogony file has been indexed for <region> (as <dataset>)?`.

Indexes the administrative regions found in the cosmogony file found in
`tests/fixtures/cosmogony/<region>/<region>.jsonl.gz` into Elasticsearch. There is an optional
dataset parameter. The dataset is, by default, set to the region, unless it is specified. This will
result in an index `munin_admin_<dataset>...`

`admins have been indexed for <region> (as <dataset>)?`.

This is a condensed format which downloads, processes, and index, so that the test background
description is more declarative than imperative. For example,

`admins have been indexed for aquitaine` would download the OSM file for Aquitaine, process it with
cosmogony, and index it in Elasticsearch with the final index name
`munin_admin_aquitaine_<timestamp>`. It would be equivalent to

```
osm file has been downloaded for aquitaine
osm file has been processed by cosmogony for aquitaine
cosmogony file has been indexed for aquitaine
```

### Addresses

`bano file has been indexed for <region> (as <dataset>)?`.

Indexes the addresses found in the BANO file found in
`tests/fixtures/bano/<region>/<region>.csv` into Elasticsearch. There is an optional
dataset parameter. The dataset is, by default, set to the region, unless it is specified. This will
result in an index `munin_addr_<dataset>...`

`addresses (bano) have been indexed for <departments> into <region> as <dataset>`.

This is a condensed format for
### Streets

### Stops

### Pois

# Scenarios
