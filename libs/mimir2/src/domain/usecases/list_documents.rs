use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;
use std::pin::Pin;

use crate::domain::{
    ports::{
        list::{Error as PortError, List, ListParameters as PrimaryParameters},
        query::Query,
    },
    usecases::{Error as UseCaseError, UseCase},
};

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
    pub parameters: PrimaryParameters,
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
impl<D: DeserializeOwned + Send + Sync + 'static> List for ListDocuments<D> {
    type Doc = D;
    fn list_documents(
        &self,
        parameters: PrimaryParameters,
    ) -> Result<Pin<Box<dyn Stream<Item = Self::Doc> + Send + 'static>>, PortError> {
        let parameters = crate::domain::ports::query::ListParameters::from(parameters);
        self.query
            .list_documents(parameters)
            .map_err(|err| PortError::DocumentRetrievalError {
                source: Box::new(err),
            })
    }
}
