use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;

use crate::domain::model::export_parameters::{
    ListParameters as ExportParameters, SearchParameters,
};
use crate::domain::model::query_parameters::ListParameters as QueryParameters;
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
    pub parameters: ExportParameters,
}

#[async_trait]
impl<D: DeserializeOwned + Send + Sync + 'static> UseCase for ListDocuments<D> {
    type Res = Pin<Box<dyn Stream<Item = D> + Send + 'static>>;
    type Param = ListDocumentsParameters;

    async fn execute(&self, param: Self::Param) -> Result<Self::Res, UseCaseError> {
        self.list_documents(param.parameters)
            .map_err(|err| UseCaseError::Execution {
                source: Box::new(err),
            })
    }
}

#[async_trait]
impl<D: DeserializeOwned + Send + Sync + 'static> Export for ListDocuments<D> {
    type Doc = D;
    fn list_documents(
        &self,
        parameters: ExportParameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, ExportError> {
        let query_parameters = QueryParameters::from(parameters);
        self.query.list_documents(query_parameters).map_err(|err| {
            ExportError::DocumentRetrievalError {
                source: Box::new(err),
            }
        })
    }

    async fn search_documents(
        &self,
        _parameters: SearchParameters,
    ) -> Result<Vec<Self::Doc>, ExportError> {
        Err(ExportError::InterfaceError {
            details: String::from("can't use ListDocuments::search_documents"),
        })
    }
}
