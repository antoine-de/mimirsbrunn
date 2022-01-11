use std::time::Duration;

use async_trait::async_trait;

use serde::de::DeserializeOwned;
use snafu::Snafu;

use crate::domain::model::query::Query;

#[derive(Debug, Clone)]
pub struct Parameters {
    pub query: Query,
    pub timeout: Option<Duration>,
}

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },
}

#[async_trait]
pub trait Get {
    type Doc: DeserializeOwned + Send + Sync + 'static;
    async fn get_documents_by_id(&self, parameters: Parameters) -> Result<Vec<Self::Doc>, Error>;
}
