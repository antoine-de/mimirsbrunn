use crate::domain::ports;
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
}

impl Error {
    pub fn from_deserialization<T: ContainerDocument>(err: serde_json::Error) -> Self {
        Self::Deserialization {
            target_type: T::static_doc_type(),
            source: err,
        }
    }
}

// Conversion from secondary ports errors

impl From<ports::secondary::list::Error> for Error {
    fn from(err: ports::secondary::list::Error) -> Self {
        match err {
            ports::secondary::list::Error::DocumentRetrievalError { source } => {
                Self::DocumentRetrievalError { source }
            }
        }
    }
}
