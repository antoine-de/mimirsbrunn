use config::Config;
use serde::{Deserialize, Serialize};
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

use crate::domain::model::configuration::root_doctype_dataset_ts;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Elasticsearch Index Configuration not found at {}", path.display()))]
    InvalidPath { path: PathBuf },

    #[snafu(display("JSON Serde Serialization Error: {}", source))]
    JsonSerialization {
        source: serde_json::Error,
        details: String,
    },
    #[snafu(display("Invalid Configuration: {} [{}]", source, details))]
    InvalidConfiguration {
        details: String,
        source: config::ConfigError,
    },
}

/// The indices create index API has 4 components, which are
/// reproduced below:
/// - Path parameter: The index name
/// - Query parameters: Things like timeout, wait for active shards, ...
/// - Request body, including
///   - Aliases (not implemented here)
///   - Mappings
///   - Settings
///   See https://www.elastic.co/guide/en/elasticsearch/reference/7.12/indices-create-index.html
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexConfiguration {
    #[serde(skip_serializing)]
    pub name: String, // name does not appear in the body of the index creation request
    #[serde(skip_serializing)]
    pub parameters: IndexParameters, // parameters don't appear in the body of the request
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<IndexSettings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mappings: Option<IndexMappings>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexSettings(serde_json::Value);

impl std::fmt::Display for IndexSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}

impl IndexSettings {
    pub fn new(value: serde_json::Value) -> IndexSettings {
        IndexSettings(value)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexMappings(serde_json::Value);

impl std::fmt::Display for IndexMappings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}

impl IndexMappings {
    pub fn new(value: serde_json::Value) -> IndexMappings {
        IndexMappings(value)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename = "snake_case")]
pub struct IndexParameters {
    pub force_merge: bool,
    pub max_number_segments: i64,
    pub wait_for_active_shards: String,
}

impl IndexConfiguration {
    // We have an input configuration that looks like
    // config
    //   ├─ container
    //   │   ├─ name: eg 'admin'
    //   │   └─ dataset: eg 'fr-idf'
    //   └─ elasticsearch
    //       ├─ mappings
    //       ├─ settings
    //       └─ parameters
    // We build the name of the index from the container name and dataset, and create a new
    // configuration that looks like
    // config
    //   └─ elasticsearch
    //       ├─ name
    //       ├─ mappings
    //       ├─ settings
    //       └─ parameters
    // Finally we turn the 'config.elasticsearch' part into an IndexConfiguration.
    pub fn new_from_config(config: Config) -> Result<Self, Error> {
        let container_name = config
            .get_string("container.name")
            .context(InvalidConfiguration {
                details: String::from("could not get key 'container.name' from configuration"),
            })?;
        let container_dataset =
            config
                .get_string("container.dataset")
                .context(InvalidConfiguration {
                    details: String::from(
                        "could not get key 'container.dataset' from configuration",
                    ),
                })?;
        let elasticsearch_name = root_doctype_dataset_ts(&container_name, &container_dataset);
        let builder = Config::builder()
            .set_default("elasticsearch.name", elasticsearch_name.clone())
            .context(InvalidConfiguration {
                details: format!(
                    "could not set key 'elasticsearch.name' to {}",
                    elasticsearch_name
                ),
            })?;

        let config = builder
            .add_source(config)
            .build()
            .context(InvalidConfiguration {
                details: format!(
                    "could not build configuration from builder for container {}",
                    container_name
                ),
            })?;

        config.get("elasticsearch").context(InvalidConfiguration {
            details: format!(
                "could not get key 'elasticsearch' from configuration for container {}",
                container_name
            ),
        })
    }
    pub fn into_json_body(self) -> Result<serde_json::Value, Error> {
        let name = self.name.clone();
        serde_json::to_value(self).context(JsonSerialization {
            details: format!("could not serialize component template {}", name),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mappings: Option<IndexMappings>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<IndexSettings>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentTemplateConfiguration {
    #[serde(skip_serializing)]
    pub name: String,
    pub template: Template,
}

impl ComponentTemplateConfiguration {
    pub fn new_from_config(config: Config) -> Result<Self, Error> {
        // FIXME Here the error can be misleading. There are two operations, one is getting
        // the key 'elasticsearch', the other is transmogrifying the config into a
        // ComponentTemplateConfiguration.
        config.get("elasticsearch").context(InvalidConfiguration {
            details: String::from("could not get key 'elasticsearch' from configuration"),
        })
    }
    pub fn into_json_body(self) -> Result<serde_json::Value, Error> {
        let name = self.name.clone();
        serde_json::to_value(self).context(JsonSerialization {
            details: format!("could not serialize component template {}", name),
        })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexTemplateConfiguration {
    #[serde(skip_serializing)]
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<Template>,
    pub composed_of: Vec<String>,
    pub index_patterns: Vec<String>,
    pub version: u32,
    pub priority: u32,
}

impl IndexTemplateConfiguration {
    pub fn new_from_config(config: Config) -> Result<Self, Error> {
        config.get("elasticsearch").context(InvalidConfiguration {
            details: String::from("could not get key 'elasticsearch' from configuration"),
        })
    }
    pub fn into_json_body(self) -> Result<serde_json::Value, Error> {
        let name = self.name.clone();
        serde_json::to_value(self).context(JsonSerialization {
            details: format!("could not serialize index template {}", name),
        })
    }
}
