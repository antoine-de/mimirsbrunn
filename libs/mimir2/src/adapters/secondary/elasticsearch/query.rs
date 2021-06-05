use async_trait::async_trait;
use futures::stream::Stream;
use serde::Serialize;

use super::ElasticsearchStorage;
use crate::domain::ports::query::{Error as QueryError, Query};

#[async_trait]
impl Query for ElasticsearchStorage {
    async fn retrieve_documents<S, D>(&self, _index: String) -> Result<S, QueryError>
    where
        D: Serialize + Send + Sync + 'static,
        S: Stream<Item = D> + Send + Sync + Unpin + 'static,
    {
        unimplemented!();
    }
}
