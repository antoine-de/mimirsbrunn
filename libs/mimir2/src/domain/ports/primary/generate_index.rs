use crate::domain::model::{
    error::Error as ModelError,
    index::{Index, IndexVisibility},
};
use crate::domain::ports::secondary::storage::Storage;
use async_trait::async_trait;
use common::document::ContainerDocument;
use config::Config;
use futures::stream::Stream;
use tracing::{info, info_span};
use tracing_futures::Instrument;

#[async_trait]
pub trait GenerateIndex {
    async fn generate_index<D: ContainerDocument + Send + Sync + 'static>(
        &self,
        config: Config,
        documents: impl Stream<Item = D> + Send + Sync + 'static,
        visibility: IndexVisibility,
    ) -> Result<Index, ModelError>;
}

#[async_trait]
impl<T> GenerateIndex for T
where
    T: Storage + Send + Sync + 'static,
{
    #[tracing::instrument(skip(self, config, documents))]
    async fn generate_index<D: ContainerDocument + Send + Sync + 'static>(
        &self,
        config: Config,
        documents: impl Stream<Item = D> + Send + Sync + 'static,
        visibility: IndexVisibility,
    ) -> Result<Index, ModelError> {
        // 1. We create the index
        // 2. We insert the document stream in that newly created index
        // 3. We publish the index
        // 4. We search for the newly created index to return it.
        let index = self
            .create_container(config.clone())
            .instrument(info_span!("Create container"))
            .await
            .map_err(|err| ModelError::IndexCreation { source: err.into() })?;

        let stats = self
            .insert_documents(index.name.clone(), documents)
            .instrument(info_span!("Insert documents"))
            .await
            .map_err(|err| ModelError::DocumentStreamInsertion { source: err.into() })?;

        info!("Index generation stats: {:?}", stats);

        self.publish_index(index.clone(), visibility)
            .instrument(info_span!("Publish index"))
            .await
            .map_err(|err| ModelError::IndexPublication { source: err.into() })?;

        // See forcemerge documentation on Elasticsearch website. Note that 'force merge should
        // only be called against an index after you have finished writing to it'.
        let force_merge: bool = config
            .get("elasticsearch.parameters.force_merge")
            .map_err(|err| ModelError::Configuration { source: err })?;

        if force_merge {
            let max_number_segments: i64 = config
                .get("elasticsearch.parameters.max_number_segments")
                .map_err(|err| ModelError::Configuration { source: err })?;

            self.force_merge(vec![index.name.clone()], max_number_segments)
                .instrument(info_span!("Force merge"))
                .await
                .map_err(|err| ModelError::IndexOptimization { source: err.into() })?;
        }

        self.find_container(index.name.clone())
            .await
            .map_err(|err| ModelError::DocumentStreamInsertion { source: err.into() })?
            .ok_or(ModelError::ExpectedIndex { index: index.name })
    }
}
