use crate::domain::model::query::Query;
use crate::domain::ports::explain::{Error, Explain, Parameters};

pub async fn explain_document<B, D>(
    backend: &B,
    query: Query,
    id: String,
    doc_type: String,
) -> Result<D, Error>
where
    B: Explain<Doc = D>,
{
    let explain_params = Parameters {
        doc_type,
        query,
        id,
    };

    backend.explain_document(explain_params).await
}
