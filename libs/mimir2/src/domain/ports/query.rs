use async_trait::async_trait;
use erased_serde::Serialize as ErasedSerialize;
use futures::stream::Stream;
use serde::Serialize;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", details))]
    DocumentRetrievalError { details: String },
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Query {
    async fn retrieve_documents<S, D>(&self, index: String) -> Result<S, Error>
    where
        D: Serialize + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + Unpin + 'static;
}

#[async_trait]
impl<'a, T: ?Sized> Query for Box<T>
where
    T: Query + Send + Sync,
{
    async fn retrieve_documents<S, D>(&self, index: String) -> Result<S, Error>
    where
        D: Serialize + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + Unpin + 'static,
    {
        (**self).retrieve_documents(index).await
    }
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait ErasedQuery {
    async fn erased_retrieve_documents(
        &self,
        index: String,
    ) -> Result<
        Box<
            dyn Stream<Item = Box<dyn ErasedSerialize + Send + Sync + 'static>>
                + Send
                + Sync
                + Unpin
                + 'static,
        >,
        Error,
    >;
}

#[async_trait]
impl Query for (dyn ErasedQuery + Send + Sync) {
    async fn retrieve_documents<S, D>(&self, index: String) -> Result<S, Error>
    where
        D: Serialize + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + Unpin + 'static,
    {
        self.retrieve_documents(index).await
    }
}

#[async_trait]
impl<T> ErasedQuery for T
where
    T: Query + Send + Sync,
{
    async fn erased_retrieve_documents(
        &self,
        index: String,
    ) -> Result<
        Box<
            dyn Stream<Item = Box<dyn ErasedSerialize + Send + Sync + 'static>>
                + Send
                + Sync
                + Unpin
                + 'static,
        >,
        Error,
    > {
        self.retrieve_documents(index).await
    }
}
