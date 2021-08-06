use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use std::convert::TryFrom;

const INDEX_ROOT: &str = "munin";

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Invalid Index Configuration: {}", details))]
    InvalidConfiguration { details: String },

    #[snafu(display("Serialization Error: {}", details))]
    Serialization { details: String },

    #[snafu(display("Invalid Name: {}", details))]
    InvalidName { details: String },
}

#[derive(Debug, Clone)]
pub struct Configuration {
    pub value: String,
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
    pub parameters: serde_json::Value,
    pub settings: serde_json::Value,
    pub mappings: serde_json::Value,
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

impl Configuration {
    pub fn normalize_index_name(self, doc_type: &str) -> Result<Self, Error> {
        let mut index_configuration = IndexConfiguration::try_from(self)?;
        index_configuration.name = root_doctype_dataset_ts(doc_type, &index_configuration.name);
        let config_str =
            serde_json::to_string(&index_configuration).map_err(|err| Error::Serialization {
                details: format!(
                    "could not serialize index configuration: {}",
                    err.to_string()
                ),
            })?;
        Ok(Configuration { value: config_str })
    }
}

pub fn root_doctype_dataset_ts(doc_type: &str, dataset: &str) -> String {
    format!(
        "{}_{}_{}_{}",
        INDEX_ROOT,
        doc_type,
        dataset,
        chrono::Utc::now().format("%Y%m%d_%H%M%S_%f")
    )
}

pub fn root_doctype_dataset(doc_type: &str, dataset: &str) -> String {
    format!("{}_{}_{}", INDEX_ROOT, doc_type, dataset,)
}

pub fn root_doctype(doc_type: &str) -> String {
    format!("{}_{}", INDEX_ROOT, doc_type,)
}

pub fn root() -> String {
    String::from(INDEX_ROOT)
}

pub fn aliases(doc_type: &str, dataset: &str) -> Vec<String> {
    vec![
        root(),
        root_doctype(doc_type),
        root_doctype_dataset(doc_type, dataset),
    ]
}

// fn doc_type_index_prefix(doc_type: &str) -> String {
//     format!("{}_{}", INDEX_ROOT, doc_type)
// }
//
// fn date_index_prefix(name: &str) -> String {
//     format!("{}_{}", name, chrono::Utc::now().format("%Y%m%d_%H%M%S_%f"))
// }

// Given an index name in the form {}_{}_{}_{}, we extract the 2nd and 3rd
// pieces which are supposed to be respectively the doc_type and the dataset.
pub fn split_index_name(name: &str) -> Result<(String, String), Error> {
    lazy_static! {
        static ref SPLIT_INDEX_NAME: Regex = Regex::new(r"[^_]+_([^_]+)_([^_]+)_*").unwrap();
    }
    if let Some(caps) = SPLIT_INDEX_NAME.captures(name) {
        let doc_type = String::from(caps.get(1).unwrap().as_str());
        let dataset = String::from(caps.get(2).unwrap().as_str());
        Ok((doc_type, dataset))
    } else {
        Err(Error::InvalidName {
            details: format!("Could not analyze index name: {}", name),
        })
    }
}
