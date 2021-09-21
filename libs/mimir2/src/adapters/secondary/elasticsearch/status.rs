use async_trait::async_trait;

use super::ElasticsearchStorage;
use crate::domain::model::status::StorageStatus;
use crate::domain::ports::secondary::status::{Error as StatusError, Status};

#[async_trait]
impl Status for ElasticsearchStorage {
    // This function delegates to elasticsearch the creation of the index. But since this
    // function returns nothing, we follow with a find index to return some details to the caller.
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
