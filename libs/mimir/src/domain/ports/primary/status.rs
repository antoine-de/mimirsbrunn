use crate::domain::{
    model::{error::Error as ModelError, status::Status as DomainStatus},
    ports::secondary::status::Status as SecondaryStatus,
};
use async_trait::async_trait;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[async_trait]
pub trait Status {
    async fn status(&self) -> Result<DomainStatus, ModelError>;
}

#[async_trait]
impl<T> Status for T
where
    T: SecondaryStatus + Send + Sync + 'static,
{
    async fn status(&self) -> Result<DomainStatus, ModelError> {
        let storage = self
            .status()
            .await
            .map_err(|err| ModelError::Status { source: err.into() })?;
        Ok(DomainStatus {
            version: VERSION.to_string(),
            storage,
        })
    }
}
