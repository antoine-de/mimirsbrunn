use elasticsearch::Elasticsearch;

pub mod internal;
pub mod query;
pub mod remote;
pub mod storage;

pub struct ElasticsearchStorage(Elasticsearch);

#[cfg(test)]
pub mod tests {

    use serde::Serialize;
    use std::convert::TryFrom;

    use super::*;

    use crate::domain::model::configuration::Configuration;
    use crate::domain::model::document::Document;
    use crate::domain::ports::remote::Remote;
    use crate::domain::ports::storage::Storage;
    use crate::utils::docker;

    #[test]
    fn should_return_invalid_configuration() {
        let config = Configuration {
            value: String::from("invalid"),
        };
        let res = internal::IndexConfiguration::try_from(config);
        assert!(res
            .unwrap_err()
            .to_string()
            .starts_with("Invalid Elasticsearch Index Configuration"));
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
        docker::initialize().await.expect("initialization");
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let _client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
    }

    #[tokio::test]
    async fn should_create_index_with_valid_configuration() {
        docker::initialize().await.expect("initialization");
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
        let config = internal::IndexConfiguration {
            name: String::from("root_obj_dataset_test-index"),
            parameters: internal::IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: internal::IndexSettings {
                value: String::from(r#"{ "index": { "number_of_shards": 1 } }"#),
            },
            mappings: internal::IndexMappings {
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
    async fn should_correctly_report_duplicate_index_when_creating_twice_the_same_index() {
        docker::initialize().await.expect("initialization");
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
        let config = internal::IndexConfiguration {
            name: String::from("root_obj_dataset_test-index-duplicate"),
            parameters: internal::IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: internal::IndexSettings {
                value: String::from(r#"{ "index": { "number_of_shards": 1 } }"#),
            },
            mappings: internal::IndexMappings {
                value: String::from(r#"{ "properties": { "value": { "type": "text" } } }"#),
            },
        };
        let config = Configuration {
            value: serde_json::to_string(&config).expect("config"),
        };
        let res = client.create_container(config.clone()).await;
        assert!(res.is_ok());
        let res = client.create_container(config).await;
        assert!(res
            .unwrap_err()
            .to_string()
            .contains("Elasticsearch Duplicate Index"));
    }

    #[tokio::test]
    async fn should_correctly_report_invalid_configuration() {
        docker::initialize().await.expect("initialization");
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
        let config = internal::IndexConfiguration {
            name: String::from("root_obj_dataset_test-index-invalid-conf"),
            parameters: internal::IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: internal::IndexSettings {
                value: String::from(r#"{ "index": }"#), // <<=== Invalid Settings
            },
            mappings: internal::IndexMappings {
                value: String::from(r#"{ "properties": { "value": { "type": "text" } } }"#),
            },
        };
        let config = Configuration {
            value: serde_json::to_string(&config).expect("config"),
        };
        let res = client.create_container(config).await;
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
        const IS_GEO_DATA: bool = false;
        const DOC_TYPE: &'static str = "test-obj";

        fn id(&self) -> String {
            self.value.clone()
        }
    }

    #[tokio::test]
    async fn should_correctly_insert_multiple_documents() {
        docker::initialize().await.expect("initialization");
        let pool = remote::connection_test_pool()
            .await
            .expect("Elasticsearch Connection Pool");
        let client = pool
            .conn()
            .await
            .expect("Elasticsearch Connection Established");
        let config = internal::IndexConfiguration {
            name: String::from("root_obj_dataset_test-index-bulk-insert"),
            parameters: internal::IndexParameters {
                timeout: String::from("10s"),
                wait_for_active_shards: String::from("1"), // only the primary shard
            },
            settings: internal::IndexSettings {
                value: String::from(r#"{ "index": { "number_of_shards": 1 } }"#), // <<=== Invalid Settings
            },
            mappings: internal::IndexMappings {
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
