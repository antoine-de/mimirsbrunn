use async_trait::async_trait;

use super::ElasticsearchStorage;
use crate::domain::model::configuration::root_doctype;
use crate::domain::ports::secondary::search::{Error, Parameters, Search};

#[async_trait]
impl Search for ElasticsearchStorage {
    type Doc = serde_json::Value;

    async fn search_documents(&self, parameters: Parameters) -> Result<Vec<Self::Doc>, Error> {
        let indices = parameters
            .doc_types
            .iter()
            .map(|idx| root_doctype(idx))
            .collect();

        self.search_documents(indices, parameters.query, parameters.result_limit)
            .await
            .map_err(|err| Error::DocumentRetrievalError { source: err.into() })
    }
}
