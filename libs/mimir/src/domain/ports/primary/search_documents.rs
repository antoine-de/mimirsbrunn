use std::time::Duration;

use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::domain::model::{error::Error as ModelError, query::Query};
use crate::domain::ports::secondary::search::{Parameters, Search};

// automock requires that the associated type be known at
// compile time. So here we import admins when we are in
// testing configuration.
#[cfg(test)]
use places::admin::Admin;

#[cfg_attr(test, mockall::automock(type Document=Admin;))]
#[async_trait]
pub trait SearchDocuments {
    type Document;

    async fn search_documents(
        &self,
        doc_types: Vec<String>,
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
        doc_types: Vec<String>,
        query: Query,
        result_limit: i64,
        timeout: Option<Duration>,
    ) -> Result<Vec<Self::Document>, ModelError> {
        self.search_documents(Parameters {
            doc_types,
            query,
            result_limit,
            timeout,
        })
        .await
        .map_err(|err| ModelError::DocumentRetrievalError { source: err.into() })
    }
}
