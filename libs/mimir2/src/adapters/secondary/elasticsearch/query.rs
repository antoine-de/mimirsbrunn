use futures::stream::Stream;
use serde_json::Value;
use std::pin::Pin;

use super::ElasticsearchStorage;
use crate::domain::model::query_parameters::QueryParameters;
use crate::domain::ports::query::{Error as QueryError, Query};

impl Query for ElasticsearchStorage {
    type Doc = Value;
    fn search_documents(
        &self,
        query_parameters: QueryParameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, QueryError> {
        self.retrieve_documents(query_parameters.containers, query_parameters.dsl)
            .map_err(|err| QueryError::DocumentRetrievalError {
                source: Box::new(err),
            })
    }
}
