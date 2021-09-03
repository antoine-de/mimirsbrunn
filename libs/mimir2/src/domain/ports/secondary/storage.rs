use async_trait::async_trait;
use futures::stream::Stream;
use snafu::Snafu;

use crate::domain::model::{
    configuration::Configuration,
    index::{Index, IndexVisibility},
    stats::InsertStats,
};
use common::document::Document;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Container Creation Error: {}", source))]
    ContainerCreationError { source: Box<dyn std::error::Error> },

    #[snafu(display("Container Deletion Error: {}", source))]
    ContainerDeletionError { source: Box<dyn std::error::Error> },

    #[snafu(display("Container Search Error: {}", source))]
    ContainerSearchError { source: Box<dyn std::error::Error> },

    #[snafu(display("Document Insertion Error: {}", source))]
    DocumentInsertionError { source: Box<dyn std::error::Error> },

    #[snafu(display("Index Refresh Error: {}", source))]
    IndexPublicationError { source: Box<dyn std::error::Error> },
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Storage {
    async fn create_container(&self, config: Configuration) -> Result<Index, Error>;

    async fn delete_container(&self, index: String) -> Result<(), Error>;

    async fn find_container(&self, index: String) -> Result<Option<Index>, Error>;

    async fn insert_documents<D, S>(
        &self,
        index: String,
        documents: S,
    ) -> Result<InsertStats, Error>
    where
        D: Document + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + Unpin + 'static;

    async fn publish_index(&self, index: Index, visibility: IndexVisibility) -> Result<(), Error>;
}

#[async_trait]
impl<'a, T: ?Sized> Storage for Box<T>
where
    T: Storage + Send + Sync,
{
    async fn create_container(&self, config: Configuration) -> Result<Index, Error> {
        (**self).create_container(config).await
    }

    async fn delete_container(&self, index: String) -> Result<(), Error> {
        (**self).delete_container(index).await
    }

    async fn find_container(&self, index: String) -> Result<Option<Index>, Error> {
        (**self).find_container(index).await
    }

    async fn insert_documents<D, S>(
        &self,
        index: String,
        documents: S,
    ) -> Result<InsertStats, Error>
    where
        D: Document + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + Unpin + 'static,
    {
        (**self).insert_documents(index, documents).await
    }

    async fn publish_index(&self, index: Index, visibility: IndexVisibility) -> Result<(), Error> {
        (**self).publish_index(index, visibility).await
    }
}
