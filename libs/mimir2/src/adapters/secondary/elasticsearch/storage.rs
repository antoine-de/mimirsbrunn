use async_trait::async_trait;
use futures::future::TryFutureExt;
use futures::stream::Stream;
use serde::Serialize;
use std::convert::TryFrom;

use super::internal;
use super::ElasticsearchStorage;
use crate::domain::model::configuration::{self, Configuration};
use crate::domain::model::index::{Index, IndexVisibility};
use crate::domain::ports::storage::{Error as StorageError, Storage};

#[async_trait]
impl Storage for ElasticsearchStorage {
    // This function delegates to elasticsearch the creation of the index. But since this
    // function returns nothing, we follow with a find index to return some details to the caller.
    async fn create_container(&self, config: Configuration) -> Result<Index, StorageError> {
        let config = internal::IndexConfiguration::try_from(config).map_err(|err| {
            StorageError::ContainerCreationError {
                details: format!("could not convert index configuration: {}", err.to_string()),
            }
        })?;
        let name = config.name.clone();
        self.create_index(config)
            .and_then(|_| {
                self.find_index(name.clone()).and_then(|res| {
                    futures::future::ready(
                        res.ok_or(internal::Error::ElasticsearchUnknownIndex { index: name }),
                    )
                })
            })
            .await
            .map_err(|err| StorageError::ContainerCreationError {
                details: format!("Could not create index: {}", err.to_string()),
            })
    }

    // FIXME Move details to impl ElasticsearchStorage.
    async fn delete_container(&self, index: String) -> Result<(), StorageError> {
        self.delete_index(index.clone())
            .await
            .map_err(|err| StorageError::ContainerDeletionError {
                details: format!("could not delete index {}: {}", index, err.to_string()),
            })
    }

    // FIXME Move details to impl ElasticsearchStorage.
    async fn find_container(&self, index: String) -> Result<Option<Index>, StorageError> {
        self.find_index(index)
            .await
            .map_err(|err| StorageError::ContainerSearchError {
                details: format!("could not find index: {}", err.to_string()),
            })
    }

    // FIXME Move details to impl ElasticsearchStorage.
    async fn insert_documents<S, D>(
        &self,
        index: String,
        documents: S,
    ) -> Result<usize, StorageError>
    where
        D: Serialize + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + Unpin + 'static,
    {
        self.insert_documents_in_index(index, documents)
            .await
            .map_err(|err| StorageError::ContainerSearchError {
                details: format!("could not insert documents: {}", err.to_string()),
            })
    }

    // Maybe all this should be run in some kind of transaction.
    async fn publish_index(
        &self,
        index: Index,
        visibility: IndexVisibility,
    ) -> Result<(), StorageError> {
        self.refresh_index(index.name.clone())
            .await
            .map_err(|err| StorageError::IndexPublicationError {
                details: format!("could not refresh index: {}", err.to_string()),
            })?;

        let previous_indices = self.get_previous_indices(&index).await.map_err(|err| {
            StorageError::IndexPublicationError {
                details: format!("could not retrieve previous indices: {}", err.to_string()),
            }
        })?;

        let base_index = configuration::root_doctype_dataset(&index.doc_type, &index.dataset);

        if !previous_indices.is_empty() {
            self.remove_alias(previous_indices.clone(), base_index.clone())
                .await
                .map_err(|err| StorageError::IndexPublicationError {
                    details: format!(
                        "could not remove old aliases, from {} to {}: {}",
                        base_index,
                        previous_indices.join(" / "),
                        err.to_string()
                    ),
                })?;
        }
        self.add_alias(vec![index.name.clone()], base_index.clone())
            .await
            .map_err(|err| StorageError::IndexPublicationError {
                details: format!(
                    "could not add alias, from {} to {}: {}",
                    base_index,
                    index.name,
                    err.to_string()
                ),
            })?;

        if visibility == IndexVisibility::Public {
            let base_index = configuration::root_doctype(&index.doc_type);
            if !previous_indices.is_empty() {
                self.remove_alias(previous_indices.clone(), base_index.clone())
                    .await
                    .map_err(|err| StorageError::IndexPublicationError {
                        details: format!(
                            "could not remove old aliases, from {} to {}: {}",
                            base_index,
                            previous_indices.join(" / "),
                            err.to_string()
                        ),
                    })?;
            }
            self.add_alias(vec![index.name.clone()], base_index.clone())
                .await
                .map_err(|err| StorageError::IndexPublicationError {
                    details: format!(
                        "could not add alias from {} to {}: {}",
                        base_index,
                        index.name,
                        err.to_string()
                    ),
                })?;
        }

        Ok(())
    }
}

#[cfg(test)]
pub mod tests {

    use super::*;

    use crate::domain::model::document::Document;

    #[test]
    #[should_panic(expected = "could not deserialize index configuration")]
    fn should_return_invalid_configuration() {
        let config = Configuration {
            value: String::from("invalid"),
        };
        let _config = IndexConfiguration::try_from(config).unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "could not parse Elasticsearch URL")]
    async fn should_return_invalid_url() {
        let _pool = connection_pool("foobar").await.unwrap();
    }

    #[tokio::test]
    async fn should_connect_to_elasticsearch() {
        let pool = connection_pool("http://localhost:9200")
            .await
            .expect("Elasticsearch Connection Pool");
        let _client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
    }

    #[tokio::test]
    async fn should_create_index_with_valid_configuration() {
        let pool = connection_pool("http://localhost:9200")
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
        let config = IndexConfiguration {
            name: String::from("root_obj_dataset_test-index"),
            parameters: IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: IndexSettings {
                value: String::from(r#"{ "index": { "number_of_shards": 1 } }"#),
            },
            mappings: IndexMappings {
                value: String::from(r#"{ "properties": { "value": { "type": "text" } } }"#),
            },
        };
        let config = Configuration {
            value: serde_json::to_string(&config).expect("config"),
        };
        let res = client.create_container(config).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    #[should_panic(
        expected = "Elasticsearch Duplicate Index: root_obj_dataset_test-index-duplicate"
    )]
    async fn should_correctly_report_duplicate_index_when_creating_twice_the_same_index() {
        let pool = connection_pool("http://localhost:9200")
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
        let config = IndexConfiguration {
            name: String::from("root_obj_dataset_test-index-duplicate"),
            parameters: IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: IndexSettings {
                value: String::from(r#"{ "index": { "number_of_shards": 1 } }"#),
            },
            mappings: IndexMappings {
                value: String::from(r#"{ "properties": { "value": { "type": "text" } } }"#),
            },
        };
        let config = Configuration {
            value: serde_json::to_string(&config).expect("config"),
        };
        let res = client.create_container(config.clone()).await;
        assert!(res.is_ok());
        let _res = client.create_container(config).await.unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "could not deserialize index configuration")]
    async fn should_correctly_report_invalid_configuration() {
        let pool = connection_pool("http://localhost:9200")
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
        let config = IndexConfiguration {
            name: String::from("root_obj_dataset_test-index-invalid-conf"),
            parameters: IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: IndexSettings {
                value: String::from(r#"{ "index": }"#), // <<=== Invalid Settings
            },
            mappings: IndexMappings {
                value: String::from(r#"{ "properties": { "value": { "type": "text" } } }"#),
            },
        };
        let config = Configuration {
            value: serde_json::to_string(&config).expect("config"),
        };
        let _res = client.create_container(config).await.unwrap();
    }

    #[derive(Serialize)]
    struct TestObj {
        value: String,
    }

    impl Document for TestObj {
        const IS_GEO_DATA: bool = false;
        const DOC_TYPE: &'static str = "test-obj";

        fn id(&self) -> String {
            self.value.clone()
        }
    }

    #[tokio::test]
    async fn should_correctly_insert_document() {
        let pool = connection_pool("http://localhost:9200")
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
        let config = IndexConfiguration {
            name: String::from("root_obj_dataset_test-index-insert"),
            parameters: IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: IndexSettings {
                value: String::from(r#"{ "index": { "number_of_shards": 1 } }"#), // <<=== Invalid Settings
            },
            mappings: IndexMappings {
                value: String::from(r#"{ "properties": { "value": { "type": "text" } } }"#),
            },
        };
        let config = Configuration {
            value: serde_json::to_string(&config).expect("config"),
        };
        let _res = client.create_container(config).await.unwrap();
        let document = TestObj {
            value: String::from("value"),
        };

        let _res = client
            .insert_document(
                String::from("test-index-insert"),
                String::from("1"),
                document,
            )
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn should_correctly_insert_bulk_documents() {
        let pool = connection_pool("http://localhost:9200")
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
        let config = IndexConfiguration {
            name: String::from("root_obj_dataset_test-index-bulk-insert"),
            parameters: IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: IndexSettings {
                value: String::from(r#"{ "index": { "number_of_shards": 1 } }"#), // <<=== Invalid Settings
            },
            mappings: IndexMappings {
                value: String::from(r#"{ "properties": { "value": { "type": "text" } } }"#),
            },
        };
        let config = Configuration {
            value: serde_json::to_string(&config).expect("config"),
        };
        let _res = client.create_container(config).await.unwrap();
        let documents = vec![
            TestObj {
                value: String::from("obj1"),
            },
            TestObj {
                value: String::from("obj2"),
            },
            TestObj {
                value: String::from("obj3"),
            },
            TestObj {
                value: String::from("obj4"),
            },
            TestObj {
                value: String::from("obj5"),
            },
            TestObj {
                value: String::from("obj6"),
            },
        ];
        let documents = futures::stream::iter(documents);

        let res = client
            .insert_documents(
                String::from("root_obj_dataset_test-index-bulk-insert"),
                documents,
            )
            .await
            .unwrap();

        assert_eq!(res, 6);
    }
}
