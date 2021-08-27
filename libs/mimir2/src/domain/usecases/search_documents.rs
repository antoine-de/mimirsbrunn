use crate::domain::model::query::Query;
use crate::domain::ports::search::{Error, Parameters, Search};
use serde::de::DeserializeOwned;

pub async fn search_documents<B, D>(
    backend: &B,
    doc_types: Vec<String>,
    query: Query,
) -> Result<Vec<D>, Error>
where
    B: Search<Doc = D>,
    D: DeserializeOwned,
{
    backend
        .search_documents(Parameters { doc_types, query })
        .await
}
