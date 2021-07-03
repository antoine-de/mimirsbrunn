/// This port defines a method to search
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
pub struct SearchParameters {
    pub doc_types: Vec<String>,
    // A valid query DSL
    pub dsl: String,
}

#[async_trait]
pub trait Search {
    type Doc: DeserializeOwned + Send + Sync + 'static;
    async fn search_documents(&self, parameters: SearchParameters)
        -> Result<Vec<Self::Doc>, Error>;
}
