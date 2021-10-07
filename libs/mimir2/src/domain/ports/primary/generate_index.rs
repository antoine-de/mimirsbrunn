use crate::domain::model::{
    error::Error as ModelError,
    index::{Index, IndexVisibility},
};
use crate::domain::ports::secondary::storage::Storage;
use async_trait::async_trait;
use common::document::ContainerDocument;
use config::Config;
use futures::stream::Stream;
use tracing::info;

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
    async fn generate_index<D: ContainerDocument + Send + Sync + 'static>(
        &self,
        config: Config,
        documents: impl Stream<Item = D> + Send + Sync + 'static,
        visibility: IndexVisibility,
    ) -> Result<Index, ModelError> {
        // 1. We modify the name of the index:
        //   currently set to the dataset, it should be something like root_doctype_dataset_timestamp
        // 2. Then we create the index
        // 3. We insert the document stream in that newly created index
        // 4. We publish the index
        // 5. We search for the newly created index to return it.

        // So we need the name of the document type.... At one point it was easy, I could use
        // a constant associated with the trait Document, and I'd be done with T::DOC_TYPE.
        // But then I had to turn this into a trait object, which forbids using associated
        // constant... So I made it a function argument.... but then the information is twice in
        // there:
        //   1) in the document type
        //   2) in the parameter doc_type
        let index = self
            .create_container(config)
            .await
            .map_err(|err| ModelError::IndexCreation { source: err.into() })?;

        let stats = self
            .insert_documents(index.name.clone(), documents)
            .await
            .map_err(|err| ModelError::DocumentStreamInsertion { source: err.into() })?;

        info!("Index generation stats: {:?}", stats);

        self.publish_index(index.clone(), visibility)
            .await
            .map_err(|err| ModelError::IndexPublication { source: err.into() })?;

        self.find_container(index.name.clone())
            .await
            .map_err(|err| ModelError::DocumentStreamInsertion { source: err.into() })?
            .ok_or(ModelError::ExpectedIndex { index: index.name })
    }
}
