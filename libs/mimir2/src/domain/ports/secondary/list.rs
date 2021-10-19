use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use snafu::Snafu;
use std::pin::Pin;

use crate::domain::model::error::Error as ModelError;

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
pub trait List<D: DeserializeOwned + Send + Sync + 'static> {
    async fn list_documents(
        &self,
        parameters: Parameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Result<D, Error>> + Send + 'static>>, Error>;
}

// Conversion from secondary ports errors
impl From<Error> for ModelError {
    fn from(err: Error) -> ModelError {
        match err {
            Error::DocumentRetrievalError { source } => {
                ModelError::DocumentRetrievalError { source }
            }
        }
    }
}
