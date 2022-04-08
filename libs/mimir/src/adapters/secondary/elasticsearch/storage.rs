use async_trait::async_trait;
use config::Config;
use futures::{
    future::TryFutureExt,
    stream::{Stream, StreamExt},
};
use serde::Serialize;
use serde_json::json;

use super::{
    configuration::{ComponentTemplateConfiguration, IndexTemplateConfiguration},
    internal, ElasticsearchStorage,
};
use crate::domain::{
    model::{
        configuration,
        configuration::{root_doctype_dataset_ts, ContainerConfig, ContainerVisibility},
        index::Index,
        stats::InsertStats,
        update::UpdateOperation,
    },
    ports::secondary::storage::{Error as StorageError, Storage},
};
use common::document::Document;

#[async_trait]
impl<'s> Storage<'s> for ElasticsearchStorage {
    // This function delegates to elasticsearch the creation of the index. But since this
    // function returns nothing, we follow with a find index to return some details to the caller.
    async fn create_container(&self, config: &ContainerConfig) -> Result<Index, StorageError> {
        let index_name = root_doctype_dataset_ts(&config.name, &config.dataset);

        self.create_index(
            &index_name,
            config.number_of_shards,
            config.number_of_replicas,
        )
        .and_then(|_| {
            self.find_index(index_name.clone()).and_then(|res| {
                futures::future::ready(res.ok_or(internal::Error::ElasticsearchUnknownIndex {
                    index: index_name.to_string(),
                }))
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
        S: Stream<Item = D> + Send + Sync + 's,
    {
        self.add_pipeline(
            include_str!(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../../config/pipeline/indexed_at.json",
            )),
            "indexed_at",
        )
        .await
        .map_err(|err| StorageError::DocumentInsertionError { source: err.into() })?;

        self.insert_documents_in_index(index, documents)
            .await
            .map(InsertStats::from)
            .map_err(|err| StorageError::DocumentInsertionError {
                source: Box::new(err),
            })
    }

    async fn update_documents<S>(
        &self,
        index: String,
        operations: S,
    ) -> Result<InsertStats, StorageError>
    where
        S: Stream<Item = (String, Vec<UpdateOperation>)> + Send + Sync + 's,
    {
        #[derive(Clone, Serialize)]
        #[serde(into = "serde_json::Value")]
        struct EsOperation(Vec<UpdateOperation>);

        #[allow(clippy::from_over_into)]
        impl Into<serde_json::Value> for EsOperation {
            fn into(self) -> serde_json::Value {
                // match self.0 {
                //     UpdateOperation::Set { ident, value } => {
                // Generate the part of the document that must be updated. For example with
                // `ident` = "properties.image" and `value` = "https://foo.jpg", this will
                // generate the following JSON:
                // { "properties": { "image": "https://foo.jpg" } }
                //         let updated_part = ident
                //             .split('.')
                //             .rev()
                //             .fold(value.into(), |val, key| json!({ key: val }));
                //
                //         json!({ "doc": updated_part })
                //     }
                // }

                self.0.into_iter().fold(json!({}), |mut result, op| {
                    match op {
                        // Adds the part of the document that must be updated to `result`. For
                        // example if at current iteration `result` has this value:
                        //   { "properties": { "review": "excellent" } }
                        // And `ident` = "properties.image", `value` = "https://foo.jpg", then this
                        // will update `result` with this value:
                        //   { "properties": { "review", "excellent", "image": "https://foo.jpg" } }
                        UpdateOperation::Set { ident, value } => {
                            // Get a reference to the position in the JSON where the value must be
                            // inserted.
                            let target = ident.split('.').fold(&mut result, |curr, key| {
                                if curr.get(key).is_none() {
                                    curr[key] = json!({});
                                }

                                &mut curr[key]
                            });

                            // Update target object with the value, in most cases this is just an
                            // empty object that was just created to construct the full path.
                            *target = value.into();
                        }
                    }

                    result
                })
            }
        }

        let operations = operations.map(|(doc_id, op)| (doc_id, EsOperation(op)));

        self.update_documents_in_index(index, operations)
            .await
            .map(InsertStats::from)
            .map_err(|err| StorageError::DocumentUpdateError {
                source: Box::new(err),
            })
    }

    // FIXME all this should be run in some kind of transaction.
    async fn publish_index(
        &self,
        index: Index,
        visibility: ContainerVisibility,
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

        if visibility == ContainerVisibility::Public {
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

        if self.config.force_merge.enabled {
            self.force_merge(&[&index.name], &self.config.force_merge)
                .await
                .map_err(|err| StorageError::ForceMergeError {
                    source: Box::new(err),
                })?;
        }

        Ok(())
    }

    async fn configure(&self, directive: String, config: Config) -> Result<(), StorageError> {
        match directive.as_str() {
            "create component template" => {
                // We build a struct from the config object,
                let config =
                    ComponentTemplateConfiguration::new_from_config(config).map_err(|err| {
                        StorageError::TemplateCreationError {
                            template: String::from("NA"),
                            source: Box::new(err),
                        }
                    })?;
                let template = config.name.clone();
                self.create_component_template(config).await.map_err(|err| {
                    StorageError::TemplateCreationError {
                        template,
                        source: Box::new(err),
                    }
                })
            }
            "create index template" => {
                let config =
                    IndexTemplateConfiguration::new_from_config(config).map_err(|err| {
                        StorageError::TemplateCreationError {
                            template: String::from("NA"),
                            source: Box::new(err),
                        }
                    })?;
                let template = config.name.clone();
                self.create_index_template(config).await.map_err(|err| {
                    StorageError::TemplateCreationError {
                        template,
                        source: Box::new(err),
                    }
                })
            }
            _ => Err(StorageError::UnrecognizedDirective { details: directive }),
        }
    }
}
