use elasticsearch::Elasticsearch;
use std::time::Duration;

pub mod configuration;
pub mod explain;
pub(super) mod internal;
pub mod list;
pub mod query;
pub mod remote;
pub mod status;
pub mod storage;

pub const ES_DEFAULT_TIMEOUT: u64 = 100; // milliseconds
pub const ES_DEFAULT_VERSION_REQ: &str = ">=7.13.0";

/// A structure wrapping around the elasticsearch's client.
#[derive(Clone, Debug)]
pub struct ElasticsearchStorage {
    /// Elasticsearch client
    pub(crate) client: Elasticsearch,
    /// Timeout used by every call to the server.
    pub timeout: Duration,
}

#[cfg(test)]
pub mod tests {

    use config::Config;
    use serde::Serialize;

    use super::*;

    use crate::domain::ports::secondary::remote::Remote;
    use crate::domain::ports::secondary::storage::Storage;
    use crate::utils::docker;
    use common::document::{ContainerDocument, Document};

    // In this test we present an invalid configuration (its actually empty) to the
    // create_container function, and expect the error message to be meaningful
    #[tokio::test]
    async fn should_return_invalid_configuration() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
            .await
            .expect("Elasticsearch Connection Established");

        let config = Config::builder().build().expect("build empty config");

        let res = client.create_container(config).await;

        drop(guard);

        assert!(res
            .unwrap_err()
            .to_string()
            .contains("Invalid Elasticsearch Index Configuration"));
    }

    #[tokio::test]
    async fn should_return_invalid_url() {
        let res = remote::connection_pool_url("foobar").await;
        assert!(res
            .unwrap_err()
            .to_string()
            .starts_with("Invalid Elasticsearch URL"));
    }

    #[tokio::test]
    async fn should_connect_to_elasticsearch() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let _client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
            .await
            .expect("Elasticsearch Connection Established");
        drop(guard);
    }

    #[tokio::test]
    async fn should_create_index_with_valid_configuration() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
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
        drop(guard);
        assert!(res.is_ok());
    }

    #[tokio::test]
    async fn should_correctly_report_invalid_configuration() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
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
        drop(guard);
        assert!(res
            .unwrap_err()
            .to_string()
            .starts_with("Container Creation Error"));
    }

    #[derive(Serialize)]
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
    async fn should_correctly_insert_multiple_documents() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
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
        drop(guard);

        assert_eq!(res.expect("insertion stats").created, 6);
    }

    #[tokio::test]
    async fn should_detect_invalid_elasticsearch_version() {
        let guard = docker::initialize()
            .await
            .expect("elasticsearch docker initialization");

        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool.conn(ES_DEFAULT_TIMEOUT, ">=9.99.99").await;

        drop(guard);

        assert_eq!(client.unwrap_err().to_string(), "Container Creation Error",);
    }
}
