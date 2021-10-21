use config::Config;
use serde::de::DeserializeOwned;
use serde::Serialize;

/// Generic document.
pub trait Document: DeserializeOwned + Serialize {
    // TODO: Do we need to use an owned string here?
    /// Unique identifier for the document.
    fn id(&self) -> String;
}

/// A type of document with a fixed type.
///
/// A collection of this kind of document has a consistent schema and can hence
/// be used to generate a container.
pub trait ContainerDocument: Document {
    fn static_doc_type() -> &'static str;

    /// Default configuration for an Elasticsearch container containing given type of document.
    ///
    /// Such a configuration is structured as follows:
    ///  - name: index name (string)
    ///  - settings: raw index settings, see
    ///    <https://www.elastic.co/guide/en/elasticsearch/reference/7.9/indices-create-index.html#create-index-settings>
    ///  - mappings: raw index mappings, see
    ///    <https://www.elastic.co/guide/en/elasticsearch/reference/7.9/mapping.html>
    ///  - parameters: query parameters at index creation, see
    ///    <https://www.elastic.co/guide/en/elasticsearch/reference/7.9/indices-create-index.html#indices-create-api-query-params>
    fn default_es_container_config() -> Config;
}
