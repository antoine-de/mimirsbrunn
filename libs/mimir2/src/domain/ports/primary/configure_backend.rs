use crate::domain::model::error::Error as ModelError;
use crate::domain::ports::secondary::storage::Storage;
use async_trait::async_trait;
use config::Config;

#[async_trait]
pub trait ConfigureBackend {
    async fn configure(&self, directive: String, config: Config) -> Result<(), ModelError>;
}

#[async_trait]
impl<T> ConfigureBackend for T
where
    T: Storage + Send + Sync + 'static,
{
    async fn configure(&self, directive: String, config: Config) -> Result<(), ModelError> {
        self.configure(directive, config)
            .await
            .map_err(|err| ModelError::BackendConfiguration { source: err.into() })
    }
}
