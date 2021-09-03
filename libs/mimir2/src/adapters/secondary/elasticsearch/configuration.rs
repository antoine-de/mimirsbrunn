use serde::{Deserialize, Serialize};
use snafu::Snafu;
use std::convert::TryFrom;

use crate::domain::model::configuration::Configuration;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Invalid Elasticsearch Index Configuration: {}", details))]
    InvalidConfiguration { details: String },
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
pub struct IndexSettings {
    pub value: serde_json::Value,
}

// FIXME A lot of work needs to go in there to type everything
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IndexMappings {
    pub value: serde_json::Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename = "snake_case")]
pub struct IndexParameters {
    pub timeout: String,
    pub wait_for_active_shards: String,
}

impl TryFrom<Configuration> for IndexConfiguration {
    type Error = Error;

    // FIXME Parameters not handled
    fn try_from(configuration: Configuration) -> Result<Self, Self::Error> {
        let Configuration { value, .. } = configuration;
        serde_json::from_str(&value).map_err(|err| Error::InvalidConfiguration {
            details: format!(
                "could not deserialize index configuration: {} / {}",
                err.to_string(),
                value
            ),
        })
    }
}
