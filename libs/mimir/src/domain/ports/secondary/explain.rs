use crate::domain::model::query::Query;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use snafu::Snafu;

#[derive(Debug, Clone)]
pub struct Parameters {
    pub doc_type: String,
    pub query: Query,
    pub id: String,
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },
}

/// This port defines a method to debug queries / settings
#[async_trait]
pub trait Explain {
    type Doc: DeserializeOwned + Send + Sync + 'static;
    async fn explain_document(&self, parameters: Parameters) -> Result<Self::Doc, Error>;
}
