use std::time::Duration;

use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::domain::{
    model::{error::Error as ModelError, query::Query},
    ports::secondary::search::{Parameters, Search},
};

// automock requires that the associated type be known at
// compile time. So here we import admins when we are in
// testing configuration.

#[async_trait]
pub trait SearchDocuments {
    type Document;

    async fn search_documents(
        &self,
        es_indices_to_search_in: Vec<String>,
        query: Query,
        result_limit: i64,
        timeout: Option<Duration>,
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
        es_indices_to_search_in: Vec<String>,
        query: Query,
        result_limit: i64,
        timeout: Option<Duration>,
    ) -> Result<Vec<Self::Document>, ModelError> {
        self.search_documents(Parameters {
            query,
            result_limit,
            timeout,
            es_indices_to_search_in,
        })
        .await
        .map_err(|err| ModelError::DocumentRetrievalError { source: err.into() })
    }
}
