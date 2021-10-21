/// This port defines a list of methods to access data
/// stored in `Storage`.
use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use snafu::Snafu;
use std::pin::Pin;

use crate::domain::model::export_parameters::{
    ExplainParameters, ListParameters, SearchParameters,
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },

    #[snafu(display("Interface Error: {}", details))]
    InterfaceError { details: String },
}

#[async_trait]
pub trait Export {
    type Doc: DeserializeOwned + Send + Sync + 'static;
    fn list_documents(
        &self,
        parameters: ListParameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, Error>;

    async fn search_documents(&self, parameters: SearchParameters)
        -> Result<Vec<Self::Doc>, Error>;

    async fn explain_document(&self, parameters: ExplainParameters) -> Result<Self::Doc, Error>;
}
