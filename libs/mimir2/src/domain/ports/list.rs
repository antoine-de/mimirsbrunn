use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use snafu::Snafu;
use std::pin::Pin;

/// This port defines a method to list documents in storage
#[derive(Debug, Clone)]
pub struct Parameters {
    pub doc_type: String,
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },
}

#[async_trait]
pub trait List {
    type Doc: DeserializeOwned + Send + Sync + 'static;

    fn list_documents(
        &self,
        parameters: Parameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, Error>;
}
