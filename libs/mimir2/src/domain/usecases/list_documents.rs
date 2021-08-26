use crate::domain::ports::list::ListParameters;
use crate::domain::ports::query::{Error, Query};
use futures::stream::Stream;
use places::MimirObject;

pub fn list_documents<B, D>(backend: &B) -> Result<impl Stream<Item = D> + Send + 'static, Error>
where
    B: Query<Doc = D>,
    D: MimirObject + 'static,
{
    backend.list_documents(
        ListParameters {
            doc_type: D::doc_type().to_string(),
        }
        .into(),
    )
}
