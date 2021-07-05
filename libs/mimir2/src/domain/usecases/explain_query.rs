use async_trait::async_trait;
use serde::de::DeserializeOwned;

use crate::domain::{
    ports::{
        explain::{Error as PortError, Explain, ExplainParameters as PrimaryParameters},
        query::Query,
    },
    usecases::{Error as UseCaseError, UseCase},
};

pub struct ExplainDocument<D> {
    pub query: Box<dyn Query<Doc = D> + Send + Sync + 'static>,
}

impl<D> ExplainDocument<D> {
    pub fn new(query: Box<dyn Query<Doc = D> + Send + Sync + 'static>) -> Self {
        ExplainDocument { query }
    }
}

#[derive(Debug)]
pub struct ExplainDocumentParameters {
    pub parameters: PrimaryParameters,
}

#[async_trait]
impl<D: DeserializeOwned + Send + Sync + 'static> UseCase for ExplainDocument<D> {
    type Res = D;
    type Param = ExplainDocumentParameters;

    async fn execute(&self, param: Self::Param) -> Result<Self::Res, UseCaseError> {
        self.explain_document(param.parameters)
            .await
            .map_err(|err| UseCaseError::Execution {
                source: Box::new(err),
            })
    }
}

#[async_trait]
impl<D: DeserializeOwned + Send + Sync + 'static> Explain for ExplainDocument<D> {
    type Doc = D;
    async fn explain_document(
        &self,
        parameters: PrimaryParameters,
    ) -> Result<Self::Doc, PortError> {
        let parameters = crate::domain::ports::query::ExplainParameters::from(parameters);
        self.query
            .explain_document(parameters)
            .await
            .map_err(|err| PortError::DocumentRetrievalError {
                source: Box::new(err),
            })
    }
}
