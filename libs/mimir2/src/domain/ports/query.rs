use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", details))]
    DocumentRetrievalError { details: String },
}

#[async_trait]
pub trait Query {
    fn retrieve_documents<D>(
        &self,
        index: String,
    ) -> Result<Box<dyn Stream<Item = D> + Unpin + 'static>, Error>
    where
        D: DeserializeOwned + 'static;
}
