use crate::domain::model::error::Error as ModelError;
use crate::domain::ports::secondary::list::{List, Parameters};
use async_trait::async_trait;
use common::document::ContainerDocument;
use futures::stream::{Stream, StreamExt};
use serde::de::DeserializeOwned;
use std::pin::Pin;

type PinnedStream<T> = Pin<Box<dyn Stream<Item = T> + Send + 'static>>;

#[async_trait]
pub trait ListDocuments<D> {
    async fn list_documents(&self) -> Result<PinnedStream<Result<D, ModelError>>, ModelError>;
}

#[async_trait]
impl<D, T> ListDocuments<D> for T
where
    D: ContainerDocument + DeserializeOwned + 'static,
    T: List + Send + Sync,
    T::Doc: Into<serde_json::Value>,
{
    async fn list_documents(&self) -> Result<PinnedStream<Result<D, ModelError>>, ModelError> {
        let doc_type = D::static_doc_type().to_string();
        let raw_documents = self.list_documents(Parameters { doc_type }).await?;

        let documents = raw_documents
            .map(|raw| raw.into())
            .map(|val| serde_json::from_value(val).map_err(ModelError::from_deserialization::<D>));

        Ok(Box::pin(documents))
    }
}
