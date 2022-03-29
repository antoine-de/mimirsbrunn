use crate::domain::{
    model::{error::Error as ModelError, query::Query},
    ports::secondary::explain::{Explain, Parameters},
};
use async_trait::async_trait;

#[async_trait]
pub trait ExplainDocument {
    type Document;

    async fn explain_document(
        &self,
        query: Query,
        id: String,
        doc_type: String,
    ) -> Result<Self::Document, ModelError>;
}

#[async_trait]
impl<T> ExplainDocument for T
where
    T: Explain + Send + Sync,
{
    type Document = T::Doc;

    async fn explain_document(
        &self,
        query: Query,
        id: String,
        doc_type: String,
    ) -> Result<Self::Document, ModelError> {
        let explain_params = Parameters {
            doc_type,
            query,
            id,
        };

        self.explain_document(explain_params)
            .await
            .map_err(|err| ModelError::DocumentRetrievalError { source: err.into() })
    }
}
