use async_trait::async_trait;
use futures::stream::Stream;
use serde_json::Value;
use std::pin::Pin;

use super::ElasticsearchStorage;
use crate::domain::model::query_parameters::{ListParameters, SearchParameters};
use crate::domain::ports::query::{Error as QueryError, Query};

#[async_trait]
impl Query for ElasticsearchStorage {
    type Doc = Value;
    async fn search_documents(
        &self,
        parameters: SearchParameters,
    ) -> Result<Vec<Self::Doc>, QueryError> {
        self.search_documents(parameters.indices, parameters.dsl)
            .await
            .map_err(|err| QueryError::DocumentRetrievalError {
                source: Box::new(err),
            })
    }

    fn list_documents(
        &self,
        parameters: ListParameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, QueryError> {
        self.list_documents(parameters.index)
            .map_err(|err| QueryError::DocumentRetrievalError {
                source: Box::new(err),
            })
    }
}
