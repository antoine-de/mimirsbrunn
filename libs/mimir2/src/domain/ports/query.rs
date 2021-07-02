use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use snafu::Snafu;
use std::pin::Pin;

use crate::domain::model::query_parameters::{ExplainParameters, ListParameters, SearchParameters};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },
}

#[async_trait]
pub trait Query {
    type Doc: DeserializeOwned + Send + Sync + 'static;

    async fn search_documents(&self, parameters: SearchParameters)
        -> Result<Vec<Self::Doc>, Error>;

    async fn explain_document(&self, parameters: ExplainParameters) -> Result<Self::Doc, Error>;

    fn list_documents(
        &self,
        parameters: ListParameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, Error>;
}
