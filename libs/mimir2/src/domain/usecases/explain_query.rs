use crate::domain::model::query::Query;
use crate::domain::ports::explain::ExplainParameters;
use crate::domain::ports::query::{Error, Query as Queriable};

pub async fn explain_document<B, D>(
    backend: &B,
    query: Query,
    id: String,
    doc_type: String,
) -> Result<D, Error>
where
    B: Queriable<Doc = D>,
{
    backend
        .explain_document(
            ExplainParameters {
                doc_type,
                query,
                id,
            }
            .into(),
        )
        .await
}
