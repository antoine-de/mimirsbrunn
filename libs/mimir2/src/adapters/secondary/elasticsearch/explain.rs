use super::ElasticsearchStorage;
use crate::domain::model::configuration::root_doctype;
use crate::domain::ports::explain::{Error, Explain, Parameters};
use async_trait::async_trait;

#[async_trait]
impl Explain for ElasticsearchStorage {
    type Doc = serde_json::Value;

    async fn explain_document(&self, parameters: Parameters) -> Result<Self::Doc, Error> {
        self.explain_search(
            root_doctype(&parameters.doc_type),
            parameters.query,
            parameters.id,
        )
        .await
        .map_err(|err| Error::DocumentRetrievalError { source: err.into() })
    }
}
