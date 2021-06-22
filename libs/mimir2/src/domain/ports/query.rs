use futures::stream::Stream;
use serde::de::DeserializeOwned;
use snafu::Snafu;
use std::pin::Pin;

use crate::domain::model::query_parameters::QueryParameters;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Document Retrieval Error: {}", source))]
    DocumentRetrievalError { source: Box<dyn std::error::Error> },
}

pub trait Query {
    type Doc: DeserializeOwned + Send + Sync + 'static;
    fn search_documents(
        &self,
        parameters: QueryParameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, Error>;
}
