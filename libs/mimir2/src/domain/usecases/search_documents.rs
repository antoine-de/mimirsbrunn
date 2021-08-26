use crate::domain::model::query::Query;
use crate::domain::ports::query::{Error, Query as Queriable};
use crate::domain::ports::search::SearchParameters;
use serde::de::DeserializeOwned;

pub async fn search_documents<B, D>(
    backend: &B,
    doc_types: Vec<String>,
    query: Query,
) -> Result<Vec<D>, Error>
where
    B: Queriable<Doc = D>,
    D: DeserializeOwned,
{
    backend
        .search_documents(SearchParameters { doc_types, query }.into())
        .await
}
