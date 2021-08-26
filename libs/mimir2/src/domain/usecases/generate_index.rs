use crate::domain::model::configuration::Configuration;
use crate::domain::model::document::Document;
use crate::domain::model::index::{Index, IndexVisibility};
use crate::domain::ports::import::Error as ImportError;
use crate::domain::ports::storage::Storage;
use futures::stream::Stream;
use tracing::info;

pub async fn generate_index<B, D, S>(
    backend: &B,
    config: Configuration,
    documents: S,
    doc_type: &str,
    visibility: IndexVisibility,
) -> Result<Index, ImportError>
where
    B: Storage + Send + Sync + 'static,
    D: Document + Send + Sync + 'static,
    S: Stream<Item = D> + Send + Sync + Unpin + 'static,
{
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

    let config = config
        .normalize_index_name(doc_type)
        .map_err(|err| ImportError::IndexCreation { source: err.into() })?;

    let index = backend
        .create_container(config)
        .await
        .map_err(|err| ImportError::IndexCreation { source: err.into() })?;

    let stats = backend
        .insert_documents(index.name.clone(), documents)
        .await
        .map_err(|err| ImportError::DocumentStreamInsertion { source: err.into() })?;

    info!("Index generation stats: {:?}", stats);

    backend
        .publish_index(index.clone(), visibility)
        .await
        .map_err(|err| ImportError::IndexPublication { source: err.into() })?;

    backend
        .find_container(index.name.clone())
        .await
        .map_err(|err| ImportError::DocumentStreamInsertion { source: err.into() })?
        .ok_or(ImportError::ExpectedIndex { index: index.name })
}
