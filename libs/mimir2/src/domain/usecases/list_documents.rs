use crate::domain::ports::list::{Error, List, Parameters};
use futures::stream::Stream;
use places::MimirObject;

pub fn list_documents<B, D>(backend: &B) -> Result<impl Stream<Item = D> + Send + 'static, Error>
where
    B: List<Doc = D>,
    D: MimirObject + 'static,
{
    let doc_type = D::doc_type().to_string();
    backend.list_documents(Parameters { doc_type })
}
