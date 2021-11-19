use crate::domain::model::configuration::ContainerConfig;
use crate::domain::model::update::UpdateOperation;
use crate::domain::model::{error::Error as ModelError, index::Index};
use crate::domain::ports::secondary::storage::Storage;
use async_trait::async_trait;
use common::document::ContainerDocument;
use futures::stream::Stream;
use tracing::{info, info_span};
use tracing_futures::Instrument;

#[async_trait]
pub trait GenerateIndex<'s> {
    /// Generate an index with provided stream of documents and publish it.
    async fn generate_index<D, S>(
        &self,
        config: &ContainerConfig,
        documents: S,
    ) -> Result<Index, ModelError>
    where
        D: ContainerDocument + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + 's;

    /// Same as generate_index but also applies a stream of updates to
    /// documents before publishing.
    async fn generate_and_update_index<D, S, U>(
        &self,
        config: &ContainerConfig,
        documents: S,
        operations: U,
    ) -> Result<Index, ModelError>
    where
        D: ContainerDocument + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + 's,
        U: Stream<Item = (String, UpdateOperation)> + Send + Sync + 's;
}

#[async_trait]
impl<'s, T> GenerateIndex<'s> for T
where
    T: Storage<'s> + Send + Sync + 'static,
{
    async fn generate_index<D, S>(
        &self,
        config: &ContainerConfig,
        documents: S,
    ) -> Result<Index, ModelError>
    where
        D: ContainerDocument + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + 's,
    {
        self.generate_and_update_index(config, documents, futures::stream::empty())
            .await
    }

    #[tracing::instrument(skip(self, config, documents, operations))]
    async fn generate_and_update_index<D, S, U>(
        &self,
        config: &ContainerConfig,
        documents: S,
        operations: U,
    ) -> Result<Index, ModelError>
    where
        D: ContainerDocument + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + 's,
        U: Stream<Item = (String, UpdateOperation)> + Send + Sync + 's,
    {
        // 1. We create the index
        // 2. We insert the document stream in that newly created index
        // 3. We update the documents with the input stream
        // 4. We publish the index
        // 5. We search for the newly created index to return it.
        let index = self
            .create_container(config)
            .instrument(info_span!("Create container"))
            .await
            .map_err(|err| ModelError::IndexCreation { source: err.into() })?;

        let stats = self
            .insert_documents(index.name.clone(), documents)
            .instrument(info_span!("Insert documents"))
            .await
            .map_err(|err| ModelError::DocumentStreamInsertion { source: err.into() })?;

        info!("Index generation stats: {:?}", stats);

        let stats = self
            .update_documents(index.name.clone(), operations)
            .instrument(info_span!("Update documents"))
            .await
            .map_err(|err| ModelError::DocumentStreamUpdate { source: err.into() })?;

        info!("Index update stats: {:?}", stats);

        self.publish_index(index.clone(), config.visibility)
            .instrument(info_span!("Publish index"))
            .await
            .map_err(|err| ModelError::IndexPublication { source: err.into() })?;

        self.find_container(index.name.clone())
            .await
            .map_err(|err| ModelError::DocumentStreamInsertion { source: err.into() })?
            .ok_or(ModelError::ExpectedIndex { index: index.name })
    }
}
