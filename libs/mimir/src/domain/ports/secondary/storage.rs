use async_trait::async_trait;
use config::Config;
use futures::stream::Stream;
use snafu::Snafu;

use crate::domain::model::{
    configuration::{ContainerConfig, ContainerVisibility},
    index::Index,
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

#[async_trait]
pub trait Storage<'s> {
    async fn create_container(&self, config: &ContainerConfig) -> Result<Index, Error>;

    async fn delete_container(&self, index: String) -> Result<(), Error>;

    async fn find_container(&self, index: String) -> Result<Option<Index>, Error>;

    async fn insert_documents<D, S>(
        &self,
        index: String,
        documents: S,
    ) -> Result<InsertStats, Error>
    where
        D: Document + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + 's;

    async fn publish_index(
        &self,
        index: Index,
        visibility: ContainerVisibility,
    ) -> Result<(), Error>;

    async fn configure(&self, directive: String, config: Config) -> Result<(), Error>;
}

#[cfg(test)]
mockall::mock! {
    Storage {
        fn create_container_(&self, config: &ContainerConfig) -> Result<Index, Error>;

        fn delete_container_(&self, index: String) -> Result<(), Error>;

        fn find_container_(&self, index: String) -> Result<Option<Index>, Error>;

        fn insert_documents_<D, S>(
            &self,
            index: String,
            documents: S,
        ) -> Result<InsertStats, Error>
        where
            D: Document + Send + Sync + 'static,
            S: Stream<Item = D> + Send + Sync + 'static;

        fn publish_index_(
            &self,
            index: Index,
            visibility: ContainerVisibility,
        ) -> Result<(), Error>;

        fn configure_(&self, directive: String, config: Config) -> Result<(), Error>;
    }
}

#[cfg(test)]
#[async_trait]
impl Storage<'static> for MockStorage {
    async fn create_container(&self, config: &ContainerConfig) -> Result<Index, Error> {
        self.create_container_(config)
    }

    async fn delete_container(&self, index: String) -> Result<(), Error> {
        self.delete_container_(index)
    }

    async fn find_container(&self, index: String) -> Result<Option<Index>, Error> {
        self.find_container_(index)
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
        self.insert_documents_(index, documents)
    }

    async fn publish_index(
        &self,
        index: Index,
        visibility: ContainerVisibility,
    ) -> Result<(), Error> {
        self.publish_index_(index, visibility)
    }

    async fn configure(&self, directive: String, config: Config) -> Result<(), Error> {
        self.configure_(directive, config)
    }
}

#[async_trait]
impl<'s, T: ?Sized> Storage<'s> for Box<T>
where
    T: Storage<'s> + Send + Sync,
{
    async fn create_container(&self, config: &ContainerConfig) -> Result<Index, Error> {
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
        S: Stream<Item = D> + Send + Sync + 's,
    {
        (**self).insert_documents(index, documents).await
    }

    async fn publish_index(
        &self,
        index: Index,
        visibility: ContainerVisibility,
    ) -> Result<(), Error> {
        (**self).publish_index(index, visibility).await
    }

    async fn configure(&self, directive: String, config: Config) -> Result<(), Error> {
        (**self).configure(directive, config).await
    }
}
