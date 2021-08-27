use async_trait::async_trait;
use futures::stream::Stream;
use snafu::Snafu;

#[cfg(test)]
use crate::domain::model::document::tests::Book;

use crate::domain::model::configuration::Configuration;
use crate::domain::model::document::Document;
use crate::domain::model::index::{Index, IndexVisibility};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Index Creation Error: {}", source))]
    // #[snafu(visibility(pub))]
    IndexCreation { source: Box<dyn std::error::Error> },

    #[snafu(display("Index Publication Error: {}", source))]
    // #[snafu(visibility(pub))]
    IndexPublication { source: Box<dyn std::error::Error> },

    #[snafu(display("Storage Connection Error: {}", source))]
    // #[snafu(visibility(pub))]
    StorageConnection { source: Box<dyn std::error::Error> },

    #[snafu(display("Document Stream Insertion Error: {}", source))]
    // #[snafu(visibility(pub))]
    DocumentStreamInsertion { source: Box<dyn std::error::Error> },

    #[snafu(display("Expected Index: {}", index))]
    ExpectedIndex { index: String },
}

/// Create index and stores documents in them.
#[async_trait]
pub trait Import {
    /// Type of document
    type Doc: Document + 'static;

    /// creates an index using the given configuration, and stores the documents.
    async fn generate_index<S>(
        &self,
        mut docs: S,
        config: Configuration,
        doc_type: &str,
        visibility: IndexVisibility,
    ) -> Result<Index, Error>
    where
        S: Stream<Item = Self::Doc> + Send + Sync + Unpin + 'static;
}

#[async_trait]
impl<'a, T: ?Sized> Import for Box<T>
where
    T: Import + Send + Sync,
{
    type Doc = T::Doc;
    async fn generate_index<S>(
        &self,
        docs: S,
        config: Configuration,
        doc_type: &str,
        visibility: IndexVisibility,
    ) -> Result<Index, Error>
    where
        S: Stream<Item = Self::Doc> + Send + Sync + Unpin + 'static,
    {
        (**self)
            .generate_index(docs, config, doc_type, visibility)
            .await
    }
}
