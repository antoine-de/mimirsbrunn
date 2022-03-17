use std::time::Duration;

use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::domain::{
    model::{error::Error as ModelError, query::Query},
    ports::secondary::get::{Get, Parameters},
};

// automock requires that the associated type be known at
// compile time. So here we import admins when we are in
// testing configuration.

#[async_trait]
pub trait GetDocuments {
    type Document;

    async fn get_documents_by_id(
        &self,
        query: Query,
        timeout: Option<Duration>,
    ) -> Result<Vec<Self::Document>, ModelError>;
}

#[async_trait]
impl<T> GetDocuments for T
where
    T: Get + Send + Sync,
    T::Doc: DeserializeOwned,
{
    type Document = T::Doc;

    async fn get_documents_by_id(
        &self,
        query: Query,
        timeout: Option<Duration>,
    ) -> Result<Vec<Self::Document>, ModelError> {
        self.get_documents_by_id(Parameters { query, timeout })
            .await
            .map_err(|err| ModelError::DocumentRetrievalError { source: err.into() })
    }
}
