use async_trait::async_trait;

use super::ElasticsearchStorage;
use crate::domain::ports::secondary::search::{Error, Parameters, Search};

#[async_trait]
impl Search for ElasticsearchStorage {
    type Doc = serde_json::Value;

    async fn search_documents(&self, parameters: Parameters) -> Result<Vec<Self::Doc>, Error> {
        self.search_documents(
            parameters.es_indices_to_search_in,
            parameters.query,
            parameters.result_limit,
            parameters.timeout,
        )
        .await
        .map_err(|err| Error::DocumentRetrievalError { source: err.into() })
    }
}
