use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::domain::model::{error::Error as ModelError, query::Query};
use crate::domain::ports::secondary::search::{Parameters, Search};

#[async_trait]
pub trait SearchDocuments {
    type Document;

    async fn search_documents(
        &self,
        doc_types: Vec<String>,
        query: Query,
    ) -> Result<Vec<Self::Document>, ModelError>;
}

#[async_trait]
impl<T> SearchDocuments for T
where
    T: Search + Send + Sync,
    T::Doc: DeserializeOwned,
{
    type Document = T::Doc;

    async fn search_documents(
        &self,
        doc_types: Vec<String>,
        query: Query,
    ) -> Result<Vec<Self::Document>, ModelError> {
        self.search_documents(Parameters { doc_types, query })
            .await
            .map_err(|err| ModelError::DocumentRetrievalError { source: err.into() })
    }
}
