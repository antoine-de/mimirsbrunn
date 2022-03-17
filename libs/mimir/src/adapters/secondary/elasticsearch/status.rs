use async_trait::async_trait;

use super::ElasticsearchStorage;
use crate::domain::{
    model::status::StorageStatus,
    ports::secondary::status::{Error as StatusError, Status},
};

#[async_trait]
impl Status for ElasticsearchStorage {
    /// Returns the status of the Elasticsearch Backend
    ///
    /// The status is a combination of the cluster's health, and its version.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// use url::Url;
    /// use mimir::domain::ports::secondary::remote::Remote;
    /// use mimir::adapters::secondary::elasticsearch;
    /// use mimir::adapters::secondary::elasticsearch::ElasticsearchStorageConfig;
    /// use mimir::domain::ports::primary::status::Status;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///   let url = Url::parse("http://localhost:9200").expect("valid url");
    ///   let client = elasticsearch::remote::connection_pool_url(&url)
    ///       .conn(ElasticsearchStorageConfig::default_testing()).await.unwrap();
    ///
    ///   let status = client.status().await.unwrap();
    /// }
    /// ```
    async fn status(&self) -> Result<StorageStatus, StatusError> {
        let cluster_health =
            self.cluster_health()
                .await
                .map_err(|err| StatusError::HealthRetrievalError {
                    source: Box::new(err),
                })?;
        let cluster_version =
            self.cluster_version()
                .await
                .map_err(|err| StatusError::VersionRetrievalError {
                    source: Box::new(err),
                })?;

        Ok(StorageStatus {
            health: cluster_health,
            version: cluster_version,
        })
    }
}
