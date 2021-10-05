use async_trait::async_trait;
use config::Config;
use futures::future::TryFutureExt;
use futures::stream::Stream;
use snafu::ResultExt;

use super::configuration::IndexConfiguration;
use super::internal;
use super::ElasticsearchStorage;
use crate::domain::model::configuration::root_doctype_dataset_ts;
use crate::domain::model::{
    configuration,
    index::{Index, IndexVisibility},
    stats::InsertStats,
};
use crate::domain::ports::secondary::storage::{Error as StorageError, Storage};
use common::document::Document;

fn build_index_configuration(config: Config) -> Result<IndexConfiguration, StorageError> {
    let container_name = config
        .get_string("container.name")
        .context(internal::InvalidConfiguration {
            details: String::from("could not get key 'container.name' from configuration"),
        })
        .map_err(|err| StorageError::ContainerCreationError {
            source: Box::new(err),
        })?;
    let container_dataset = config
        .get_string("container.dataset")
        .context(internal::InvalidConfiguration {
            details: String::from("could not get key 'container.dataset' from configuration"),
        })
        .map_err(|err| StorageError::ContainerCreationError {
            source: Box::new(err),
        })?;
    let elasticsearch_name = root_doctype_dataset_ts(&container_name, &container_dataset);
    let builder = Config::builder()
        .set_default("elasticsearch.name", elasticsearch_name.clone())
        .context(internal::InvalidConfiguration {
            details: format!(
                "could not set key 'elasticsearch.name' to {}",
                elasticsearch_name
            ),
        })
        .map_err(|err| StorageError::ContainerCreationError {
            source: Box::new(err),
        })?;

    let config = builder
        .add_source(config)
        .build()
        .context(internal::InvalidConfiguration {
            details: String::from("could not build configuration from builder"),
        })
        .map_err(|err| StorageError::ContainerCreationError {
            source: Box::new(err),
        })?;

    config
        .get("elasticsearch")
        .context(internal::InvalidConfiguration {
            details: String::from("could not get key 'elasticsearch' from configuration"),
        })
        .map_err(|err| StorageError::ContainerCreationError {
            source: Box::new(err),
        })
}

#[async_trait]
impl Storage for ElasticsearchStorage {
    // This function delegates to elasticsearch the creation of the index. But since this
    // function returns nothing, we follow with a find index to return some details to the caller.
    async fn create_container(&self, config: Config) -> Result<Index, StorageError> {
        let config = build_index_configuration(config)?;
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
                source: Box::new(err),
            })
    }

    async fn delete_container(&self, index: String) -> Result<(), StorageError> {
        self.delete_index(index.clone())
            .await
            .map_err(|err| StorageError::ContainerDeletionError {
                source: Box::new(err),
            })
    }

    async fn find_container(&self, index: String) -> Result<Option<Index>, StorageError> {
        self.find_index(index)
            .await
            .map_err(|err| StorageError::ContainerSearchError {
                source: Box::new(err),
            })
    }

    // FIXME Explain why we call add_pipeline
    async fn insert_documents<D, S>(
        &self,
        index: String,
        documents: S,
    ) -> Result<InsertStats, StorageError>
    where
        D: Document + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + 'static,
    {
        self.add_pipeline(
            String::from(include_str!(
                "../../../../../../config/pipeline/indexed_at.json"
            )),
            String::from("indexed_at"),
        )
        .await
        .map_err(|err| StorageError::DocumentInsertionError {
            source: Box::new(err),
        })?;
        self.insert_documents_in_index(index, documents)
            .await
            .map(InsertStats::from)
            .map_err(|err| StorageError::DocumentInsertionError {
                source: Box::new(err),
            })
    }

    // FIXME all this should be run in some kind of transaction.
    async fn publish_index(
        &self,
        index: Index,
        visibility: IndexVisibility,
    ) -> Result<(), StorageError> {
        self.refresh_index(index.name.clone())
            .await
            .map_err(|err| StorageError::IndexPublicationError {
                source: Box::new(err),
            })?;

        let previous_indices = self.get_previous_indices(&index).await.map_err(|err| {
            StorageError::IndexPublicationError {
                source: Box::new(err),
            }
        })?;

        let doctype_dataset_alias =
            configuration::root_doctype_dataset(&index.doc_type, &index.dataset);
        self.update_alias(
            doctype_dataset_alias,
            &[index.name.clone()],
            &previous_indices,
        )
        .await
        .map_err(|err| StorageError::IndexPublicationError {
            source: Box::new(err),
        })?;

        if visibility == IndexVisibility::Public {
            let doctype_alias = configuration::root_doctype(&index.doc_type);
            self.update_alias(
                doctype_alias.clone(),
                &[index.name.clone()],
                &previous_indices,
            )
            .await
            .map_err(|err| StorageError::IndexPublicationError {
                source: Box::new(err),
            })?;

            let root_alias = configuration::root();
            self.update_alias(root_alias, &[index.name.clone()], &previous_indices)
                .await
                .map_err(|err| StorageError::IndexPublicationError {
                    source: Box::new(err),
                })?;
        }

        for index_name in previous_indices {
            self.delete_container(index_name).await?;
        }

        Ok(())
    }
}
