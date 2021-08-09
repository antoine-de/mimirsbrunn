use async_trait::async_trait;
use elasticsearch::http::transport::{
    BuildError as TransportBuilderError, SingleNodeConnectionPool, TransportBuilder,
};
use elasticsearch::Elasticsearch;
use snafu::{ResultExt, Snafu};
use url::Url;

use super::ElasticsearchStorage;
use crate::domain::ports::remote::{Error as RemoteError, Remote};

const ES_KEY: &str = "ELASTICSEARCH_URL";
const ES_TEST_KEY: &str = "ELASTICSEARCH_TEST_URL";

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Invalid Elasticsearch URL: {}, {}", details, source))]
    InvalidUrl {
        details: String,
        source: url::ParseError,
    },

    #[snafu(display("Elasticsearch Connection Error: {}", source))]
    ElasticsearchConnectionError { source: TransportBuilderError },

    #[snafu(display("Missing Environment Variable {}: {}", key, source))]
    MissingEnvironmentVariable {
        key: String,
        source: std::env::VarError,
    },
}

#[async_trait]
impl Remote for SingleNodeConnectionPool {
    type Conn = ElasticsearchStorage;

    /// Use the connection to create a client.
    async fn conn(self) -> Result<Self::Conn, RemoteError> {
        let transport = TransportBuilder::new(self)
            .build()
            .context(ElasticsearchConnectionError)
            .map_err(|err| RemoteError::Connection {
                source: Box::new(err),
            })?;
        let client = Elasticsearch::new(transport);
        Ok(ElasticsearchStorage::new(client))
    }
}

/// Opens a connection to elasticsearch given a url
pub async fn connection_pool_url(url: &str) -> Result<SingleNodeConnectionPool, Error> {
    let url = Url::parse(url).context(InvalidUrl {
        details: String::from(url),
    })?;
    let pool = SingleNodeConnectionPool::new(url);
    Ok(pool)
}

/// Open a connection to elasticsearch
pub async fn connection_pool() -> Result<SingleNodeConnectionPool, Error> {
    let url = std::env::var(ES_KEY).context(MissingEnvironmentVariable {
        key: String::from(ES_KEY),
    })?;
    connection_pool_url(&url).await
}

/// Open a connection to a test elasticsearch
pub async fn connection_test_pool() -> Result<SingleNodeConnectionPool, Error> {
    let url = std::env::var(ES_TEST_KEY).context(MissingEnvironmentVariable {
        key: String::from(ES_TEST_KEY),
    })?;
    connection_pool_url(&url).await
}
