use crate::domain::ports::secondary::list::{Error, List, Parameters};
use async_trait::async_trait;
use futures::stream::Stream;
use places::MimirObject;

#[async_trait]
pub trait ListDocuments {
    type Document;

    fn list_documents(
        &self,
    ) -> Result<Box<dyn Stream<Item = Self::Document> + Send + 'static>, Error>;
}

impl<T> ListDocuments for T
where
    T: List,
    T::Doc: MimirObject + 'static,
{
    type Document = T::Doc;

    fn list_documents(
        &self,
    ) -> Result<Box<dyn Stream<Item = Self::Document> + Send + 'static>, Error> {
        let doc_type = T::Doc::doc_type().to_string();
        Ok(Box::new(self.list_documents(Parameters { doc_type })?))
    }
}
