use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use snafu::Snafu;

// FIXME The code in this module should probably not be in 'configuration.rs'
//
/// Prefix used for all indexes that mimir interacts with.
pub const INDEX_ROOT: &str = "munin";

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Invalid Index Configuration: {}", details))]
    InvalidConfiguration { details: String },

    #[snafu(display("Serialization Error: {}", details))]
    Serialization { details: String },

    #[snafu(display("Invalid Name: {}", details))]
    InvalidName { details: String },
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ContainerConfig {
    pub name: String,
    pub dataset: String,
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
