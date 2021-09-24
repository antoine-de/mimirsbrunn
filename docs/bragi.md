# Bragi

Bragi is a web application providing a REST interface for querying a geospatial backend.
Bragi currently only works with Elasticsearch.

## Getting Started

Bragi is a part of Mimirsbrunn, and so it is built like any other rust project. Assuming
you have setup a rust environment,

TODO Note about minimal rust version here and checking the rust version.

```sh
git clone git@github.com:CanalTP/mimirsbrunn.git
cd mimirsbrunn
cargo build --release
```

This will create an executable in `./target/release/bragi`

## Usage

Before using Bragi, you need to have an Elasticsearch backend available with data
indexed beforehand. Instructions on indexing are found [there](docs/indexing.md).

To start Bragi, you need to specify a configuration directory, and a run_mode.

For example, to start bragi for testing, and to look for the configuration in the
repository's config directory, run the following command.

```
./target/release/bragi -c ./config -m testing run
```

## Configuration

Bragi's configuration is split in two sections:
- one part contains parameters needed to tune the performance of the query
- and the rest.

The reason to split the configuration is that the part related to the query is
used in other contexts than Bragi.

The part related to the query is found in the `query` folder under the config base directory,
specified at the command line with `-c <config>`. The rest is found in `bragi`. So typically,
you will find:

```
bragi -c ./config

config
  |-- query
  |     |-- default.toml
  |
  |-- bragi
        |-- default.toml
	|-- testing.toml
	|-- prod.toml
	|-- [...]
```

Bragi uses a layered approach to configuration. It will start by reading the
`default.toml` configuration, and override it with the file corresponding to
the run mode. So, in the previous example, if you run `bragi -c ./config -m
testing run`, it will read `./config/bragi/default.toml`, and override any
value with those found in `./config/bragi/testing.toml`. You can still override
some settings with local values, using a `./config/bragi/local.toml`.

The Bragi configuration allows you to specify
* where the log files are stored,
* how to connect to the elasticsearch backend 
* and on what address / port to serve Bragi

Here is an example:

```toml
[logging]
path = "./logs"

[elasticsearch]
url = "http://localhost:9201"

[service]
host = "0.0.0.0"
port = "6010"
```

## REST API

### Forward Geocoding

Get a list of places (administrative regions, streets, ...) that best match your query string

**URL** : `/api/v1/autocomplete/`

**Method** : `GET`

**Query Parameters**

TODO How to specify negative long lat ?
+---------------+--------------------------+-------------------------+----------------------+
| name          | type                     | description             | example              |
+---------------+--------------------------+-------------------------+----------------------+
| q             | string                   | query string            | lond                 |
+---------------+--------------------------+-------------------------+----------------------+
| lat           | double (optional)        | latitude. Used to boost | 45.3456              |
|               |                          | results in the vicinity |                      |
+---------------+--------------------------+-------------------------+----------------------+
| lon           | double (optional)        | longitude.              | 2.4554               |
+---------------+--------------------------+-------------------------+----------------------+
| datasets      | strings, comma separated | restrics the search to  |                      |
|               | (optional)               | the given datasets. (1) |                      |
+---------------+--------------------------+-------------------------+----------------------+

TODO Finish

pub shape: Option<String>,
pub shape_scope: Option<Vec<String>>,
pub datasets: Option<Vec<String>>,
pub timeout: u32, // timeout to Elasticsearch in milliseconds

## Success Response

**Code** : `200 OK`

**Content examples**

For a User with ID 1234 on the local database where that User has saved an
email address and name information.

```json
{
    "id": 1234,
    "first_name": "Joe",
    "last_name": "Bloggs",
    "email": "joe25@example.com"
}
```

For a user with ID 4321 on the local database but no details have been set yet.

```json
{
    "id": 4321,
    "first_name": "",
    "last_name": "",
    "email": ""
}
```

## Notes

* If the User does not have a `UserInfo` instance when requested then one will
  be created for them.

### Reverse Geocoding

### Status

### Features


## Architecture

Bragi is a web application providing a REST interface for querying
Elasticsearch in the context of Mimirksbrunn. By that I mean it can only be used
to query data that have been previously stored in Elasticsearch by one of
mimirsbrunn's binary.

Since Mimirsbrunn follows an hexagonal architecture, one part of bragi must be
an adapter (aka controller).  That is, one component of bragi must _adapt_ the
input data from the http / REST interface to the primary port.

So Bragi's code is divided in three sections:
1. The part of the code dedicated to its configuration, and its execution.
2. The part of the code common with other primary adapters
3. The part of the code specific to Bragi's primary adapter.

### Execution

The part of the code dealing with command line arguments, configuration, and
launching the web server.

We find that code in `src/bragi`:

- `src/bragi/main.rs` contains code for dealing with command-line, and delegate
  the subsequent execution to:
- `src/bragi/server.rs` performs the following:
    1. initializes the logging / tracing,
    2. creates a configuration (see `src/settings.rs`)
    3. initializes a connection to the backend storage (Elasticsearch)
    4. creates the API object responsible for the server's functionality
    5. calls warp to serve the API.
- `src/bragi/settings.rs` merges the information from the command-line, from
  stored configuration files, and from environment variable to create a
  configuration.

### Common

This is the code that will be available to all primary adapters.

Found in `libs/mimir2/src/adapters/primary/common`:

- `common/settings.rs` contains the query settings used to parameterize the
  query dsl sent by bragi to Elasticsearch. The settings are read from file
  configuration (or possibly sent by POST in debugging / test environments)

- `common/filters.rs` contains a `struct Filter` which contains all the user
  supplied information to tweak the query dsl.

- `common/dsl.rs` contains the code to create a query dsl.

### REST Adapter

Found in `libs/mimir2/src/adapters/primary/bragi`:

- `bragi/routes`: each REST endpoint constitute a route

- `bragi/api`: All the structures used by the REST API to receive and transmit
  data, including for example response body, error

- `bragi/handlers`: ultimately a REST endpoint (or route) must make a call to
  the backend, and this is the role of the handler.


## Configuration

Bragi's configuration is split in two
1. One section deal with the web server and the connection to Elasticsearch,
2. The other is about the parameters that go into building a query for Elasticsearch.

The reason for splitting the configuration is that you may need one and not the
other: You don't necessarily need to go through Bragi's web server to query
Elasticsearch.

So the first part of the configuration is in `config/bragi`, while the other is
in `config/query`.

### Bragi's configuration

We use a layered approach to the configuration. First there is a default
configuration, which is always read (`default.toml`). Then, depending on the
setting (ie dev, prod, test, ...) we override with a corresponding
configuration (`dev.toml`, `prod.toml`, ...). Finally we override with
environment variables and command line arguments.

## Misc

How does a route works?

Let's look at the main autocomplete endpoint...

```rust
    let api = routes::forward_geocoder()
        .and(routes::with_client(client))
        .and(routes::with_settings(settings.query))
        .and_then(handlers::forward_geocoder)
        .recover(routes::report_invalid)
        .with(warp::trace::request());
```

Bragi uses the [warp](https://crates.io/crates/warp) web server framework,
which combines *Filter*s to achieve its functionality. 

So the first Filter is `forward_geocoder` which is

```
pub fn forward_geocoder() -> impl Filter<Extract = (InputQuery,), Error = Rejection> + Clone {
    warp::get()
        .and(path_prefix())
        .and(warp::path("autocomplete"))
        .and(forward_geocoder_query())
}
```

That is this filter will go through if the request is an HTTP GET, if the path is prefixed...

```rust
fn path_prefix() -> impl Filter<Extract = (), Error = Rejection> + Clone {
    path!("api" / "v1" / ..).boxed()
}
```

and then followed by 'autocomplete', and finally if we can extract valid query parameters.

If this is the case, then we pass in to subsequent filters the backend (client), and a data structure to 
construct the query DSL (`settings.query`). At that point, the next layer is handed 3 arguments:
* input query parameters from the first filter (`routes::forward_geocoder`)
* elasticsearch client connection (`routes::with_client`)
* query settings (`routes::with_settings`)

so the handler, which does the actual request to the primary port, builds a
search response, and pass it to the next layer. We'll see later if things fall through

```rust
pub async fn forward_geocoder(
    params: InputQuery,
    client: ElasticsearchStorage,
    settings: settings::QuerySettings,
) -> Result<impl warp::Reply, warp::Rejection> {

    let q = params.q.clone();
    let filters = filters::Filters::from(params);
    let dsl = dsl::build_query(&q, filters, &["fr"], &settings);

    match client.search_documents([...], Query::QueryDSL(dsl)).await
    {
        Ok(res) => {
            let resp = SearchResponseBody::from(res);
            Ok(with_status(json(&resp), StatusCode::OK))
        }
	[...]
    }
}
```

The last two filters of the autocomplete route are

```rust
.recover(routes::report_invalid)
.with(warp::trace::request());
```

and they ensure that any error happening in any of the preceding layer is correctly handled, and 
that we trace queries.
