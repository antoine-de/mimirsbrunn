use crate::utils::deserialize::deserialize_duration;
use elasticsearch::Elasticsearch;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;
use url::Url;

pub mod configuration;
pub mod explain;
pub(super) mod internal;
pub mod list;
pub mod models;
pub mod query;
pub mod remote;
pub mod status;
pub mod storage;

/// A structure wrapping around the elasticsearch's client.
#[derive(Clone, Debug)]
pub struct ElasticsearchStorage {
    /// Elasticsearch client
    pub(crate) client: Elasticsearch,
    /// Client configuration
    pub config: ElasticsearchStorageConfig,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct ElasticsearchStorageConfig {
    pub url: Url,
    #[serde(deserialize_with = "deserialize_duration")]
    pub timeout: Duration,
    pub version_req: String,
    pub scroll_chunk_size: u64,
    pub scroll_pit_alive: String,
    pub insertion_concurrent_requests: usize,
    pub insertion_chunk_size: usize,
}

impl Default for ElasticsearchStorageConfig {
    /// We retrieve the elasticsearch configuration from ./config/elasticsearch/default.
    fn default() -> Self {
        let config = common::config::config_from(
            &PathBuf::from("config"),
            &["elasticsearch"],
            None,
            None,
            None,
        );

        config
            .expect("cannot build the configuration for testing from config")
            .get("elasticsearch")
            .expect("expected elasticsearch section in configuration from config")
    }
}

impl ElasticsearchStorageConfig {
    pub fn default_testing() -> Self {
        let config_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../config");

        let config = common::config::config_from(
            config_dir.as_path(),
            &["elasticsearch"],
            "testing",
            "MIMIR_TEST",
            None,
        );

        config
            .unwrap_or_else(|_| {
                panic!(
                    "cannot build the configuration for testing from {}",
                    config_dir.display(),
                )
            })
            .get("elasticsearch")
            .unwrap_or_else(|_| {
                panic!(
                    "expected elasticsearch section in configuration from {}",
                    config_dir.display(),
                )
            })
    }
}

#[cfg(test)]
pub mod tests {

    use config::Config;
    use serde::{Deserialize, Serialize};
    use serial_test::serial;

    use super::*;

    use crate::domain::ports::secondary::remote::Remote;
    use crate::domain::ports::secondary::storage::Storage;
    use crate::utils::docker;
    use common::document::{ContainerDocument, Document};

    // In this test we present an invalid configuration (its actually empty) to the
    // create_container function, and expect the error message to be meaningful
    #[tokio::test]
    #[serial]
    async fn should_return_invalid_configuration() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let client = remote::connection_test_pool()
            .conn(ElasticsearchStorageConfig::default_testing())
            .await
            .expect("Elasticsearch Connection Established");

        let config = Config::builder().build().expect("build empty config");

        let res = client.create_container(config).await;

        assert!(res
            .unwrap_err()
            .to_string()
            .contains("Container Creation Error: Invalid Configuration"));
    }

    #[tokio::test]
    #[serial]
    async fn should_connect_to_elasticsearch() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let _client = remote::connection_test_pool()
            .conn(ElasticsearchStorageConfig::default_testing())
            .await
            .expect("Elasticsearch Connection Established");
    }

    #[tokio::test]
    #[serial]
    async fn should_create_index_with_valid_configuration() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");
        let client = remote::connection_test_pool()
            .conn(ElasticsearchStorageConfig::default_testing())
            .await
            .expect("Elasticsearch Connection Established");
        let config = config::Config::builder()
            .add_source(config::File::from_str(
                r#"{
                    "container": {
                        "name": "foo",
                        "dataset": "bar"
                    },
                    "elasticsearch": {
                        "parameters": {
                            "timeout": "10s",
                            "force_merge": true,
                            "max_number_segments": 1,
                            "wait_for_active_shards": "1"
                        },
                        "settings": {
                            "index": {
                                "number_of_shards": 1,
                                "number_of_replicas": 1
                            }
                        },
                        "mappings": {
                            "properties": {
                                "value": {
                                    "type": "text"
                                }
                            }
                        }
                    }
                }"#,
                config::FileFormat::Json,
            ))
            .build()
            .expect("valid configuration");
        let res = client.create_container(config).await;
        assert!(res.is_ok());
    }

    #[tokio::test]
    #[serial]
    async fn should_correctly_report_invalid_configuration() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let client = remote::connection_test_pool()
            .conn(ElasticsearchStorageConfig::default_testing())
            .await
            .expect("Elasticsearch Connection Established");
        let config = config::Config::builder()
            .add_source(config::File::from_str(
                r#"{
                    "container": {
                        "name": "foo",
                        "dataset": "bar"
                    },
                    "elasticsearch": {
                        "parameters": {
                            "timeout": "10s",
                            "wait_for_active_shards": "1"
                        },
                        "settings": {
                            "indx": {
                                "number_of_shards": 1,
                                "number_of_replicas": 1
                            }
                        },
                        "mappings": {
                            "properties": {
                                "value": {
                                    "type": "text"
                                }
                            }
                        }
                    }
                }"#,
                config::FileFormat::Json,
            ))
            .build()
            .expect("valid configuration");
        let res = client.create_container(config).await;
        assert!(res
            .unwrap_err()
            .to_string()
            .starts_with("Container Creation Error"));
    }

    #[derive(Deserialize, Serialize)]
    struct TestObj {
        value: String,
    }

    impl Document for TestObj {
        fn id(&self) -> String {
            self.value.clone()
        }
    }

    impl ContainerDocument for TestObj {
        fn static_doc_type() -> &'static str {
            "test-obj"
        }

        fn default_es_container_config() -> config::Config {
            config::Config::builder()
                .set_default("container.name", Self::static_doc_type())
                .unwrap()
                .set_default("container.dataset", "default")
                .unwrap()
                .set_default("elasticsearch.parameters.timeout", "10s")
                .unwrap()
                .set_default("elasticsearch.parameters.force_merge", true)
                .unwrap()
                .set_default("elasticsearch.parameters.max_number_segments", 1)
                .unwrap()
                .set_default("elasticsearch.parameters.wait_for_active_shards", "1")
                .unwrap()
                .add_source(config::File::from_str(
                    r#"{ "elasticsearch": { "settings": { "index": { "number_of_shards": 1, "number_of_replicas": 1 } } } }"#,
                    config::FileFormat::Json,
                ))
                .add_source(config::File::from_str(
                    r#"{ "elasticsearch": { "mappings": { "properties": { "value": { "type": "text" } } } } }"#,
                    config::FileFormat::Json,
                ))
                .build()
                .expect("invalid container configuration for TestObj")
        }
    }

    #[tokio::test]
    #[serial]
    async fn should_correctly_insert_multiple_documents() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let client = remote::connection_test_pool()
            .conn(ElasticsearchStorageConfig::default_testing())
            .await
            .expect("Elasticsearch Connection Established");

        client
            .create_container(TestObj::default_es_container_config())
            .await
            .expect("container creation");

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
            .await;

        assert_eq!(res.expect("insertion stats").created, 6);
    }

    #[tokio::test]
    #[serial]
    async fn should_detect_invalid_elasticsearch_version() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let client = remote::connection_test_pool()
            .conn(ElasticsearchStorageConfig {
                version_req: ">=9.99.99".to_string(),
                ..ElasticsearchStorageConfig::default_testing()
            })
            .await;

        assert!(client
            .unwrap_err()
            .to_string()
            .contains("Elasticsearch Invalid version"));
    }
}