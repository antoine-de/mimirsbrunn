/// This port defines a method to list documents in storage
use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use snafu::Snafu;
use std::pin::Pin;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },

    #[snafu(display("Interface Error: {}", details))]
    InterfaceError { details: String },
}

#[derive(Debug, Clone)]
pub struct ListParameters {
    pub doc_type: String,
}

#[async_trait]
pub trait List {
    type Doc: DeserializeOwned + Send + Sync + 'static;
    fn list_documents(
        &self,
        parameters: ListParameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, Error>;
}
