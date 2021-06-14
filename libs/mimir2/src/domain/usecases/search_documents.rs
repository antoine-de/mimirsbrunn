use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;

use crate::domain::model::query_parameters::QueryParameters;
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
    pub query_parameters: QueryParameters,
}

#[async_trait]
impl<D: DeserializeOwned + Send + Sync + 'static> UseCase for SearchDocuments<D> {
    type Res = Box<dyn Stream<Item = D> + Send + Sync + 'static>;
    type Param = SearchDocumentsParameters;

    async fn execute(&self, param: Self::Param) -> Result<Self::Res, UseCaseError> {
        self.search_documents(param.query_parameters)
            .map_err(|err| UseCaseError::Execution {
                details: format!("Could not search documents: {}", err.to_string()),
            })
    }
}

#[async_trait]
impl<D: DeserializeOwned + Send + Sync + 'static> Export for SearchDocuments<D> {
    type Doc = D;
    fn search_documents(
        &self,
        query_parameters: QueryParameters,
    ) -> Result<Box<dyn Stream<Item = Self::Doc> + Send + Sync + 'static>, ExportError> {
        self.query
            .search_documents(query_parameters)
            .map_err(|err| ExportError::DocumentRetrievalError {
                source: Box::new(err),
            })
    }
}

#[cfg(test)]
pub mod tests {

    use serde::Serialize;

    use super::{SearchDocuments, SearchDocumentsParameters};
    use crate::domain::model::configuration::Configuration;
    use crate::domain::model::document::Document;
    use crate::domain::model::index::{Index, IndexStatus, IndexVisibility};
    use crate::domain::ports::storage::MockErasedStorage;
    use crate::domain::usecases::UseCase;

    #[derive(Serialize)]
    struct TestObj {
        value: String,
    }

    impl Document for TestObj {
        const IS_GEO_DATA: bool = false;
        const DOC_TYPE: &'static str = "test-obj";

        fn id(&self) -> String {
            self.value.clone()
        }
    }

    #[tokio::test]
    async fn should_get_index_from_configuration_and_documents_stream() {
        let mut storage = MockErasedStorage::new();
        let index = Index {
            name: String::from("test"),
            doc_type: String::from("obj"),
            dataset: String::from("x"),
            status: IndexStatus::NotAvailable,
            docs_count: 0,
        };
        let index_cl = index.clone();
        storage
            .expect_erased_create_container()
            .times(1)
            .return_once(move |_| Ok(index));
        storage
            .expect_erased_insert_documents()
            .times(1)
            .return_once(move |_, _| Ok(1));
        storage
            .expect_erased_find_container()
            .times(1)
            .return_once(move |_| Ok(Some(index_cl)));
        storage
            .expect_erased_publish_index()
            .times(1)
            .return_once(move |_, _| Ok(()));
        let usecase = SearchDocuments::new(Box::new(storage));

        let config = Configuration {
            value: String::from(
                r#"{ "name": "test-index", "parameters": { "timeout": "10s", "wait_for_active_shards": "1" }, "settings": { "value": "NA" }, "mappings": { "value": "NA" } }"#,
            ),
        };

        let stream = futures::stream::iter(vec![TestObj {
            value: String::from("value"),
        }]);

        let param = SearchDocumentsParameters {
            config,
            documents: Box::new(stream),
            visibility: IndexVisibility::Public,
        };

        let result = usecase.execute(param).await;
        assert!(result.is_ok());
    }
}
