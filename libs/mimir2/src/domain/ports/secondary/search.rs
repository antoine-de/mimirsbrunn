use crate::domain::model::query::Query;
use async_trait::async_trait;
use serde::de::DeserializeOwned;
use snafu::Snafu;

#[derive(Debug, Clone)]
pub struct Parameters {
    pub doc_types: Vec<String>,
    pub query: Query,
    pub result_limit: Option<i64>,
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },
}

#[async_trait]
pub trait Search {
    type Doc: DeserializeOwned + Send + Sync + 'static;
    async fn search_documents(&self, parameters: Parameters) -> Result<Vec<Self::Doc>, Error>;
}
