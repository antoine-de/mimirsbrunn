use crate::document::ContainerDocument;
use config::Config;

/// Default configuration for an ElasticSearch container containing given type
/// of document.
///
/// Such a configuration is structured as follows:
///  - name: index name (string)
///  - settings: raw index settings, see
///    <https://www.elastic.co/guide/en/elasticsearch/reference/7.9/indices-create-index.html#create-index-settings>
///  - mappings: raw index mappings, see
///    <https://www.elastic.co/guide/en/elasticsearch/reference/7.9/mapping.html>
///  - parameters: query parameters at index creation, see
///    <https://www.elastic.co/guide/en/elasticsearch/reference/7.9/indices-create-index.html#indices-create-api-query-params>
pub trait DefaultEsContainerConfig: ContainerDocument {
    fn default_es_container_config() -> Config;
}
