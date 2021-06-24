use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;

use crate::domain::model::query_parameters::QueryParameters;
use crate::domain::ports::export::{Error as ExportError, Export};
use crate::domain::ports::query::Query;
use crate::domain::usecases::{Error as UseCaseError, UseCase};

// FIXME Maybe need two use cases.... one for
pub struct ListDocuments<D> {
    pub query: Box<dyn Query<Doc = D> + Send + Sync + 'static>,
}

impl<D> ListDocuments<D> {
    pub fn new(query: Box<dyn Query<Doc = D> + Send + Sync + 'static>) -> Self {
        ListDocuments { query }
    }
}

pub struct ListDocumentsParameters {
    pub query_parameters: QueryParameters,
}

#[async_trait]
impl<D: DeserializeOwned + Send + Sync + 'static> UseCase for ListDocuments<D> {
    type Res = Pin<Box<dyn Stream<Item = D> + Send + 'static>>;
    type Param = ListDocumentsParameters;

    async fn execute(&self, param: Self::Param) -> Result<Self::Res, UseCaseError> {
        self.query
            .list_documents(param.query_parameters)
            .map_err(|err| UseCaseError::Execution {
                source: Box::new(err),
            })
    }
}
