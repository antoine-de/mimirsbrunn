use async_trait::async_trait;
use futures::stream::Stream;
use std::pin::Pin;

use super::ElasticsearchStorage;
use crate::domain::ports::query::{
    Error as QueryError, ExplainParameters, ListParameters, Query, SearchParameters,
};

#[async_trait]
impl Query for ElasticsearchStorage {
    type Doc = serde_json::Value;
    async fn search_documents(
        &self,
        parameters: SearchParameters,
    ) -> Result<Vec<Self::Doc>, QueryError> {
        self.search_documents(parameters.indices, parameters.query)
            .await
            .map_err(|err| QueryError::DocumentRetrievalError {
                source: Box::new(err),
            })
    }

    async fn explain_document(
        &self,
        parameters: ExplainParameters,
    ) -> Result<Self::Doc, QueryError> {
        self.explain_search(parameters.index, parameters.query, parameters.id)
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
