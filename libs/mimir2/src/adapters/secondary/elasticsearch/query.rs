use async_trait::async_trait;
use futures::stream::Stream;
use serde::de::DeserializeOwned;

use super::ElasticsearchStorage;
use crate::domain::ports::query::{Error as QueryError, Query};

#[async_trait]
impl Query for ElasticsearchStorage {
    fn retrieve_documents<D>(
        &self,
        index: String,
    ) -> Result<Box<dyn Stream<Item = D> + Unpin + 'static>, QueryError>
    where
        D: DeserializeOwned + 'static,
    {
        unimplemented!()
    }
}
