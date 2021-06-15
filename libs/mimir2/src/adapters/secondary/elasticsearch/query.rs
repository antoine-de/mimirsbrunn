use futures::stream::Stream;

use super::ElasticsearchStorage;
use crate::domain::model::query_parameters::QueryParameters;
use crate::domain::ports::query::{Error as QueryError, Query};

impl Query for ElasticsearchStorage {
    type Doc = String;
    fn search_documents(
        &self,
        query_parameters: QueryParameters,
    ) -> Result<Box<dyn Stream<Item = Self::Doc> + 'static>, QueryError> {
        let stream = self
            .retrieve_documents(query_parameters.containers, query_parameters.dsl)
            .map_err(|err| QueryError::DocumentRetrievalError {
                source: Box::new(err),
            })?;
        Ok(Box::new(stream))
    }
}
