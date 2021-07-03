use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use snafu::Snafu;
use std::pin::Pin;

use crate::domain::model::configuration::root_doctype;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },
}

#[derive(Debug, Clone)]
pub struct ListParameters {
    pub index: String,
}

impl From<crate::domain::ports::list::ListParameters> for ListParameters {
    fn from(input: crate::domain::ports::list::ListParameters) -> Self {
        // We get a doc_type, and we need to translate that into the name of an index.
        ListParameters {
            index: root_doctype(&input.doc_type),
        }
    }
}

#[derive(Debug, Clone)]
pub struct SearchParameters {
    pub indices: Vec<String>, // if you want to target all indices, use vec![munin]
    pub dsl: String,          // if you want to target all documents, use { match_all: {} }
}

impl From<crate::domain::ports::search::SearchParameters> for SearchParameters {
    fn from(input: crate::domain::ports::search::SearchParameters) -> Self {
        SearchParameters {
            indices: input
                .doc_types
                .iter()
                .map(|doc_type| root_doctype(&doc_type))
                .collect(),
            dsl: input.dsl,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ExplainParameters {
    pub index: String, // if you want to target all indices, use vec![munin]
    pub dsl: String,   // if you want to target all documents, use { match_all: {} }
    pub id: String,
}

impl From<crate::domain::ports::explain::ExplainParameters> for ExplainParameters {
    fn from(input: crate::domain::ports::explain::ExplainParameters) -> Self {
        ExplainParameters {
            index: root_doctype(&input.doc_type),
            dsl: input.dsl,
            id: input.id,
        }
    }
}

#[async_trait]
pub trait Query {
    type Doc: DeserializeOwned + Send + Sync + 'static;

    async fn search_documents(&self, parameters: SearchParameters)
        -> Result<Vec<Self::Doc>, Error>;

    async fn explain_document(&self, parameters: ExplainParameters) -> Result<Self::Doc, Error>;

    fn list_documents(
        &self,
        parameters: ListParameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, Error>;
}
