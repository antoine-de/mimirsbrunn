/// This port defines a method to debug queries / settings
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },

    #[snafu(display("Interface Error: {}", details))]
    InterfaceError { details: String },
}

#[derive(Debug, Clone)]
pub struct ExplainParameters {
    pub doc_type: String,
    // A valid query DSL
    pub dsl: String,
    pub id: String,
}

#[async_trait]
pub trait Explain {
    type Doc: DeserializeOwned + Send + Sync + 'static;
    async fn explain_document(&self, parameters: ExplainParameters) -> Result<Self::Doc, Error>;
}
