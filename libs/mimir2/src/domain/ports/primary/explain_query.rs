use crate::domain::model::query::Query;
use crate::domain::ports::secondary::explain::{Error, Explain, Parameters};
use async_trait::async_trait;

#[async_trait]
pub trait ExplainDocument {
    type Document;

    async fn explain_document(
        &self,
        query: Query,
        id: String,
        doc_type: String,
    ) -> Result<Self::Document, Error>;
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
    ) -> Result<Self::Document, Error> {
        let explain_params = Parameters {
            doc_type,
            query,
            id,
        };

        self.explain_document(explain_params).await
    }
}
