use async_trait::async_trait;
use elasticsearch::http::transport::{
    BuildError as TransportBuilderError, SingleNodeConnectionPool, TransportBuilder,
};
use elasticsearch::Elasticsearch;
use snafu::{ResultExt, Snafu};
use url::Url;

use super::ElasticsearchStorage;
use crate::domain::ports::remote::{Error as RemoteError, Remote};

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Invalid URL: {}, {}", details, source))]
    InvalidUrl {
        details: String,
        source: url::ParseError,
    },

    /// Elasticsearch Build Error
    #[snafu(display("Elasticsearch Connection Error: {}", source))]
    ElasticsearchConnectionError { source: TransportBuilderError },
}

#[async_trait]
impl Remote for SingleNodeConnectionPool {
    type Conn = ElasticsearchStorage;

    /// Use the connection to create a client.
    async fn conn(self) -> Result<Self::Conn, RemoteError> {
        let transport = TransportBuilder::new(self)
            .disable_proxy()
            .build()
            .context(ElasticsearchConnectionError)
            .map_err(|err| RemoteError::Connection {
                details: err.to_string(),
            })?;
        let client = Elasticsearch::new(transport);
        Ok(ElasticsearchStorage::new(client))
    }
}

/// Open a connection to elasticsearch
pub async fn connection_pool(url: &str) -> Result<SingleNodeConnectionPool, Error> {
    let url = Url::parse(url).context(InvalidUrl {
        details: String::from("could not parse Elasticsearch URL"),
    })?;
    let pool = SingleNodeConnectionPool::new(url);
    Ok(pool)
}
