use common::document::ContainerDocument;
use lazy_static::lazy_static;
use regex::Regex;
use snafu::Snafu;
use std::{marker::PhantomData, path::PathBuf};

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

/// Configuration for a container. As we can't expose any implementation
/// specific APIs here, we avoid storing any mappings, ES settings & cie.
#[derive(Clone)]
pub struct ContainerConfiguration<D: ContainerDocument> {
    /// Container name, which will be generated from document type by default.
    pub dataset: String,

    /// Path where to load the configuration from (if not using default).
    ///
    /// For an ElasticSearch backend, this path must be a directory containing
    /// any combination of this two files:
    ///
    ///  - *mappings.json*: replaces default mappings
    ///  - *settings.json*: replaces default settings
    ///
    /// See https://www.elastic.co/guide/en/elasticsearch/reference/7.12/indices-create-index.html
    /// for more details.
    pub path: Option<PathBuf>,

    /// Number of replicas for this container data.
    pub nb_replicas: Option<usize>,

    /// Number of shards one replica will be split into.
    pub nb_shards: Option<usize>,

    /// This struct doesn't own any document, so we must be carreful not to
    /// indicate any ownership to the compiler. Also, there is no lifetime
    /// relationship between ContainerConfiguration<D> and D, so we can use
    /// `fn(D) -> D` marker which is invariant over D.
    _phantom: PhantomData<fn(D) -> D>,
}

impl<D: ContainerDocument> ContainerConfiguration<D> {
    pub fn new(dataset: String) -> Self {
        Self {
            dataset,
            path: None,
            nb_replicas: None,
            nb_shards: None,
            _phantom: PhantomData::default(),
        }
    }

    /// Override default config using input path.
    pub fn with_config_path(mut self, path: PathBuf) -> Self {
        self.path = Some(path);
        self
    }

    pub fn name(&self) -> String {
        root_doctype_dataset_ts(D::static_doc_type(), &self.dataset)
    }
}

impl<D: ContainerDocument> Default for ContainerConfiguration<D> {
    fn default() -> Self {
        Self::new("default".to_string())
    }
}

impl<D: ContainerDocument> std::fmt::Debug for ContainerConfiguration<D> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ContainerDocument")
            .field("dataset", &self.dataset)
            .field("path", &self.path)
            .field("nb_replicas", &self.nb_replicas)
            .field("nb_shards", &self.nb_shards)
            .finish()
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
