
## A word of caution

Keep in mind that while you're developing with mimirsbrunn, you will probably run tests at one point
or another. You must then ensure that you're not wiping a production Elasticsearch in the process.
When you are running tests, eg `cargo test`, the tests will read the
`config/elasticsearch/default.toml` file, and override its values with ones found in
`config/elasticsearch/testing.md`. Make sure the `elasticsearch.url` value specified is of no
importance.

* An introduction to mimirsbrunn's [software architecture](architecture.md)

* An [explanation](bragi.md) of Bragi, a REST API to query Elasticsearch

* A [presentation](concepts.md) of some of the concepts used in the context of Mimirsbrunn.

* [indexing](indexing.md)

* [indices](indices.md)

* [testing](testing.md)

# Tools

## import2mimir

## autocomplete

# Testing


