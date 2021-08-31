use crate::domain::model::query::Query;
use crate::domain::ports::secondary::search::{Error, Parameters, Search};
use async_trait::async_trait;
use serde::de::DeserializeOwned;

#[async_trait]
pub trait SearchDocuments {
    type Document;

    async fn search_documents(
        &self,
        doc_types: Vec<String>,
        query: Query,
    ) -> Result<Vec<Self::Document>, Error>;
}

#[async_trait]
impl<T> SearchDocuments for T
where
    T: Search + Send + Sync,
    T::Doc: DeserializeOwned,
{
    type Document = T::Doc;

    async fn search_documents(
        &self,
        doc_types: Vec<String>,
        query: Query,
    ) -> Result<Vec<Self::Document>, Error> {
        self.search_documents(Parameters { doc_types, query }).await
    }
}
