use futures::stream::Stream;
use serde::de::DeserializeOwned;

use super::ElasticsearchStorage;
use crate::domain::model::query_parameters::QueryParameters;
use crate::domain::ports::export::{Error as ExportError, Export};

impl Export for ElasticsearchStorage {
    type Doc = String;
    fn search_documents(
        &self,
        query_parameters: QueryParameters,
    ) -> Result<Box<dyn Stream<Item = Self::Doc> + Send + Sync + 'static>, ExportError> {
        let stream = self
            .retrieve_documents(query_parameters.containers, query_parameters.dsl)
            .map_err(|err| ExportError::DocumentRetrievalError {
                source: Box::new(err),
            })?;
        Ok(Box::new(stream))
    }
}
