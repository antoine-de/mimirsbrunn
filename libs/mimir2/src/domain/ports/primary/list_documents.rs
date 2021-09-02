use crate::domain::model::document::ContainerDocument;
use crate::domain::model::error::Error;
use crate::domain::ports::secondary::list::{List, Parameters};
use async_trait::async_trait;
use futures::stream::{Stream, StreamExt};

#[async_trait]
pub trait ListDocuments<D> {
    fn list_documents(
        &self,
    ) -> Result<Box<dyn Stream<Item = Result<D, Error>> + Send + 'static>, Error>;
}

impl<D, T> ListDocuments<D> for T
where
    D: ContainerDocument + serde::de::DeserializeOwned + 'static,
    T: List,
    T::Doc: Into<serde_json::Value>,
{
    fn list_documents(
        &self,
    ) -> Result<Box<dyn Stream<Item = Result<D, Error>> + Send + 'static>, Error> {
        let doc_type = D::static_doc_type().to_string();
        let raw_documents = self.list_documents(Parameters { doc_type })?;

        let documents = raw_documents
            .map(|raw| raw.into())
            .map(|val| serde_json::from_value(val).map_err(Error::from_deserialization::<D>));

        Ok(Box::new(documents))
    }
}
