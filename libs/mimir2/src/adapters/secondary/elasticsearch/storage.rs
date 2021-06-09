use async_trait::async_trait;
use futures::future::TryFutureExt;
use futures::stream::Stream;
use serde::Serialize;
use std::convert::TryFrom;

use super::internal;
use super::ElasticsearchStorage;
use crate::domain::model::configuration::{self, Configuration};
use crate::domain::model::index::{Index, IndexVisibility};
use crate::domain::ports::storage::{Error as StorageError, Storage};

#[async_trait]
impl Storage for ElasticsearchStorage {
    // This function delegates to elasticsearch the creation of the index. But since this
    // function returns nothing, we follow with a find index to return some details to the caller.
    async fn create_container(&self, config: Configuration) -> Result<Index, StorageError> {
        let config = internal::IndexConfiguration::try_from(config).map_err(|err| {
            StorageError::ContainerCreationError {
                details: format!("could not convert index configuration: {}", err.to_string()),
            }
        })?;
        let name = config.name.clone();
        self.create_index(config)
            .and_then(|_| {
                self.find_index(name.clone()).and_then(|res| {
                    futures::future::ready(
                        res.ok_or(internal::Error::ElasticsearchUnknownIndex { index: name }),
                    )
                })
            })
            .await
            .map_err(|err| StorageError::ContainerCreationError {
                details: format!("Could not create index: {}", err.to_string()),
            })
    }

    // FIXME Move details to impl ElasticsearchStorage.
    async fn delete_container(&self, index: String) -> Result<(), StorageError> {
        self.delete_index(index.clone())
            .await
            .map_err(|err| StorageError::ContainerDeletionError {
                details: format!("could not delete index {}: {}", index, err.to_string()),
            })
    }

    // FIXME Move details to impl ElasticsearchStorage.
    async fn find_container(&self, index: String) -> Result<Option<Index>, StorageError> {
        self.find_index(index)
            .await
            .map_err(|err| StorageError::ContainerSearchError {
                details: format!("could not find index: {}", err.to_string()),
            })
    }

    // FIXME Move details to impl ElasticsearchStorage.
    async fn insert_documents<S, D>(
        &self,
        index: String,
        documents: S,
    ) -> Result<usize, StorageError>
    where
        D: Serialize + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + Unpin + 'static,
    {
        self.add_pipeline(
            String::from(include_str!(
                "../../../../../../config/pipeline/indexed_at.json"
            )),
            String::from("indexed_at"),
        )
        .await
        .map_err(|err| StorageError::DocumentInsertionError {
            details: format!(
                "could not create {} pipeline: {}",
                "indexed_at",
                err.to_string()
            ),
        })?;
        self.insert_documents_in_index(index, documents)
            .await
            .map_err(|err| StorageError::DocumentInsertionError {
                details: format!("could not insert documents: {}", err.to_string()),
            })
    }

    // Maybe all this should be run in some kind of transaction.
    async fn publish_index(
        &self,
        index: Index,
        visibility: IndexVisibility,
    ) -> Result<(), StorageError> {
        self.refresh_index(index.name.clone())
            .await
            .map_err(|err| StorageError::IndexPublicationError {
                details: format!("could not refresh index: {}", err.to_string()),
            })?;

        let previous_indices = self.get_previous_indices(&index).await.map_err(|err| {
            StorageError::IndexPublicationError {
                details: format!("could not retrieve previous indices: {}", err.to_string()),
            }
        })?;

        let base_index = configuration::root_doctype_dataset(&index.doc_type, &index.dataset);

        if !previous_indices.is_empty() {
            self.remove_alias(previous_indices.clone(), base_index.clone())
                .await
                .map_err(|err| StorageError::IndexPublicationError {
                    details: format!(
                        "could not remove old aliases, from {} to {}: {}",
                        base_index,
                        previous_indices.join(" / "),
                        err.to_string()
                    ),
                })?;
        }
        self.add_alias(vec![index.name.clone()], base_index.clone())
            .await
            .map_err(|err| StorageError::IndexPublicationError {
                details: format!(
                    "could not add alias, from {} to {}: {}",
                    base_index,
                    index.name,
                    err.to_string()
                ),
            })?;

        if visibility == IndexVisibility::Public {
            let base_index = configuration::root_doctype(&index.doc_type);
            if !previous_indices.is_empty() {
                self.remove_alias(previous_indices.clone(), base_index.clone())
                    .await
                    .map_err(|err| StorageError::IndexPublicationError {
                        details: format!(
                            "could not remove old aliases, from {} to {}: {}",
                            base_index,
                            previous_indices.join(" / "),
                            err.to_string()
                        ),
                    })?;
            }
            self.add_alias(vec![index.name.clone()], base_index.clone())
                .await
                .map_err(|err| StorageError::IndexPublicationError {
                    details: format!(
                        "could not add alias from {} to {}: {}",
                        base_index,
                        index.name,
                        err.to_string()
                    ),
                })?;
        }

        Ok(())
    }
}
