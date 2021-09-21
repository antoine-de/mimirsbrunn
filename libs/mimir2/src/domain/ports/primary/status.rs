use crate::domain::model::{error::Error as ModelError, status::StorageStatus};
use crate::domain::ports::secondary::status::Status as SecondaryStatus;
use async_trait::async_trait;

#[async_trait]
pub trait Status {
    async fn status(&self) -> Result<StorageStatus, ModelError>;
}

#[async_trait]
impl<T> Status for T
where
    T: SecondaryStatus + Send + Sync + 'static,
{
    async fn status(&self) -> Result<StorageStatus, ModelError> {
        self.status()
            .await
            .map_err(|err| ModelError::Status { source: err.into() })
    }
}
