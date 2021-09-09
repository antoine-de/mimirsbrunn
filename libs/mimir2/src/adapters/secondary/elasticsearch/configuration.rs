use common::document::ContainerDocument;
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use snafu::Snafu;
use std::{
    convert::TryFrom,
    path::{Path, PathBuf},
};
use tracing::log::warn;

use crate::domain::model::configuration::ContainerConfiguration;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Invalid Elasticsearch Index Configuration: {}", details))]
    InvalidConfiguration { details: String },
    #[snafu(display("Elasticsearch Index Configuration not found at {}", path.display()))]
    InvalidPath { path: PathBuf },
}

impl From<serde_json::Error> for Error {
    fn from(source: serde_json::Error) -> Self {
        Self::InvalidConfiguration {
            details: source.to_string(),
        }
    }
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
    pub name: String,
    pub parameters: IndexParameters,
    pub settings: IndexSettings,
    pub mappings: IndexMappings,
}

// FIXME A lot of work needs to go in there to type everything
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexSettings(serde_json::Value);

impl std::fmt::Display for IndexSettings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}

// FIXME A lot of work needs to go in there to type everything
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexMappings(serde_json::Value);

impl std::fmt::Display for IndexMappings {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::result::Result<(), std::fmt::Error> {
        self.0.fmt(f)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename = "snake_case")]
pub struct IndexParameters {
    pub timeout: String,
    pub wait_for_active_shards: String,
}

fn load_settings_value<S: DeserializeOwned>(path: &Path) -> Option<serde_json::Result<S>> {
    match std::fs::File::open(path) {
        Ok(file) => Some(serde_json::from_reader(file)),
        Err(err) => {
            if err.kind() == std::io::ErrorKind::NotFound {
                warn!("Could not open `{}`: {}", path.display(), err);
            }

            None
        }
    }
}

impl<D: ContainerDocument> TryFrom<ContainerConfiguration<D>> for IndexConfiguration {
    type Error = Error;

    fn try_from(config: ContainerConfiguration<D>) -> Result<Self, Self::Error> {
        if !config.path.as_ref().map(|p| p.exists()).unwrap_or(true) {
            return Err(Error::InvalidPath {
                path: config.path.unwrap(),
            });
        }

        let mut es_config = Self {
            name: config.name(),
            settings: (config.path.as_ref())
                .and_then(|path| load_settings_value(&path.join("settings.json")))
                .unwrap_or_else(|| serde_json::from_str(D::default_es_settings()))?,
            mappings: (config.path.as_ref())
                .and_then(|path| load_settings_value(&path.join("mappings.json")))
                .unwrap_or_else(|| serde_json::from_str(D::default_es_mappings()))?,
            parameters: IndexParameters {
                timeout: "10s".to_string(),
                wait_for_active_shards: "1".to_string(),
            },
        };

        if let Some(nb_replicas) = config.nb_replicas {
            es_config.settings.0["number_of_replicas"] = nb_replicas.to_string().into();
        }

        if let Some(nb_shards) = config.nb_shards {
            es_config.settings.0["number_of_shards"] = nb_shards.to_string().into();
        }

        Ok(es_config)
    }
}
