use serde::{Deserialize, Serialize};
use snafu::Snafu;
use std::path::PathBuf;

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
    pub timeout: String,                // TODO How should we set this value
    pub wait_for_active_shards: String, // TODO How should we set this value
}
