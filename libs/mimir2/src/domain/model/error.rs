use common::document::ContainerDocument;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Failed to parse {} document: {}", target_type, source))]
    Deserialization {
        target_type: &'static str,
        source: serde_json::Error,
    },
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },

    #[snafu(display("Index Creation Error: {}", source))]
    IndexCreation { source: Box<dyn std::error::Error> },

    #[snafu(display("Index Publication Error: {}", source))]
    IndexPublication { source: Box<dyn std::error::Error> },

    #[snafu(display("Storage Connection Error: {}", source))]
    StorageConnection { source: Box<dyn std::error::Error> },

    #[snafu(display("Document Stream Insertion Error: {}", source))]
    DocumentStreamInsertion { source: Box<dyn std::error::Error> },

    #[snafu(display("Expected Index: {}", index))]
    ExpectedIndex { index: String },

    #[snafu(display("Status Error: {}", source))]
    Status { source: Box<dyn std::error::Error> },
}

impl Error {
    pub fn from_deserialization<T: ContainerDocument>(err: serde_json::Error) -> Self {
        Self::Deserialization {
            target_type: T::static_doc_type(),
            source: err,
        }
    }
}
