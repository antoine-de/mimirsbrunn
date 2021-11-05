use async_trait::async_trait;
use config::Config;
use futures::stream::Stream;
use snafu::Snafu;

use crate::domain::model::{
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

    #[snafu(display("Force Merge Error: {}", source))]
    ForceMergeError { source: Box<dyn std::error::Error> },

    #[snafu(display("Template '{}' creation error: {}", template, source))]
    TemplateCreationError {
        template: String,
        source: Box<dyn std::error::Error>,
    },

    #[snafu(display("Unrecognized directive: {}", details))]
    UnrecognizedDirective { details: String },
}

#[cfg_attr(test, mockall::automock)]
#[async_trait]
pub trait Storage {
    async fn create_container(&self, config: Config) -> Result<Index, Error>;

    async fn delete_container(&self, index: String) -> Result<(), Error>;

    async fn find_container(&self, index: String) -> Result<Option<Index>, Error>;

    async fn insert_documents<D, S>(
        &self,
        index: String,
        documents: S,
    ) -> Result<InsertStats, Error>
    where
        D: Document + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + 'static;

    async fn publish_index(&self, index: Index, visibility: IndexVisibility) -> Result<(), Error>;

    async fn force_merge(&self, indices: Vec<String>, max_num_segments: i64) -> Result<(), Error>;

    async fn configure(&self, directive: String, config: Config) -> Result<(), Error>;
}

#[async_trait]
impl<'a, T: ?Sized> Storage for Box<T>
where
    T: Storage + Send + Sync,
{
    async fn create_container(&self, config: Config) -> Result<Index, Error> {
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
        S: Stream<Item = D> + Send + Sync + 'static,
    {
        (**self).insert_documents(index, documents).await
    }

    async fn publish_index(&self, index: Index, visibility: IndexVisibility) -> Result<(), Error> {
        (**self).publish_index(index, visibility).await
    }

    async fn force_merge(&self, indices: Vec<String>, max_num_segments: i64) -> Result<(), Error> {
        (**self).force_merge(indices, max_num_segments).await
    }

    async fn configure(&self, directive: String, config: Config) -> Result<(), Error> {
        (**self).configure(directive, config).await
    }
}