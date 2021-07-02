/// Implementation of `Export` for searching documents.
use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;

use crate::domain::model::export_parameters::{
    ListParameters, SearchParameters as ExportParameters,
};
use crate::domain::model::query_parameters::SearchParameters as QueryParameters;
use crate::domain::ports::export::{Error as ExportError, Export};
use crate::domain::ports::query::Query;
use crate::domain::usecases::{Error as UseCaseError, UseCase};

pub struct SearchDocuments<D> {
    pub query: Box<dyn Query<Doc = D> + Send + Sync + 'static>,
}

impl<D> SearchDocuments<D> {
    pub fn new(query: Box<dyn Query<Doc = D> + Send + Sync + 'static>) -> Self {
        SearchDocuments { query }
    }
}

pub struct SearchDocumentsParameters {
    pub parameters: ExportParameters,
}

pub struct ExplainDocumentsParameters {
    pub parameters: crate::domain::model::export_parameters::ExplainParameters,
}

#[async_trait]
impl<D: DeserializeOwned + Send + Sync + 'static> UseCase for SearchDocuments<D> {
    type Res = Vec<D>;
    type Param = SearchDocumentsParameters;

    async fn execute(&self, param: Self::Param) -> Result<Self::Res, UseCaseError> {
        self.search_documents(param.parameters)
            .await
            .map_err(|err| UseCaseError::Execution {
                source: Box::new(err),
            })
    }
}

#[async_trait]
impl<D: DeserializeOwned + Send + Sync + 'static> Export for SearchDocuments<D> {
    type Doc = D;
    // FIXME If this ain't a code smell, then what is!!?
    fn list_documents(
        &self,
        _parameters: ListParameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, ExportError> {
        Err(ExportError::InterfaceError {
            details: String::from("can't use SearchDocuments::list_documents"),
        })
    }

    async fn search_documents(
        &self,
        parameters: ExportParameters,
    ) -> Result<Vec<Self::Doc>, ExportError> {
        let query_parameters = QueryParameters::from(parameters);
        self.query
            .search_documents(query_parameters)
            .await
            .map_err(|err| ExportError::DocumentRetrievalError {
                source: Box::new(err),
            })
    }

    async fn explain_document(
        &self,
        parameters: crate::domain::model::export_parameters::ExplainParameters,
    ) -> Result<Self::Doc, ExportError> {
        let query_parameters =
            crate::domain::model::query_parameters::ExplainParameters::from(parameters);
        self.query
            .explain_document(query_parameters)
            .await
            .map_err(|err| ExportError::DocumentRetrievalError {
                source: Box::new(err),
            })
    }
}
