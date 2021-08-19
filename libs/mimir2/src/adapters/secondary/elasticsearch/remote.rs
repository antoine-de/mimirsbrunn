use async_trait::async_trait;
use elasticsearch::http::headers::HeaderMap;
use elasticsearch::http::transport::{
    BuildError as TransportBuilderError, SingleNodeConnectionPool, TransportBuilder,
};
use elasticsearch::http::Method;
use elasticsearch::Elasticsearch;
use lazy_static::lazy_static;
use semver::{Version, VersionReq};
use serde_json::Value;
use snafu::{ResultExt, Snafu};
use url::Url;

use super::ElasticsearchStorage;
use crate::domain::ports::remote::{Error as RemoteError, Remote};

pub const ES_KEY: &str = "ELASTICSEARCH_URL";
pub const ES_TEST_KEY: &str = "ELASTICSEARCH_TEST_URL";
pub const ES_VERSION_REQ: &str = ">=7.11.0";

#[derive(Debug, Snafu)]
#[snafu(visibility(pub))]
pub enum Error {
    #[snafu(display("Invalid Elasticsearch URL: {}, {}", details, source))]
    InvalidUrl {
        details: String,
        source: url::ParseError,
    },

    #[snafu(display("Elasticsearch Transport Error: {}", source))]
    ElasticsearchTransportError { source: TransportBuilderError },

    #[snafu(display("Elasticsearch Connection Error: {}", source))]
    ElasticsearchConnectionError { source: elasticsearch::Error },

    #[snafu(display("Missing Environment Variable {}: {}", key, source))]
    MissingEnvironmentVariable {
        key: String,
        source: std::env::VarError,
    },

    /// Elasticsearch Deserialization Error
    #[snafu(display("JSON Elasticsearch Deserialization Error: {}", source))]
    JsonDeserializationError { source: elasticsearch::Error },

    /// Elasticsearch Exception
    #[snafu(display("Elasticsearch Exception: {}", msg))]
    ElasticsearchException { msg: String },

    /// Invalid JSON Value
    #[snafu(display("JSON Deserialization Invalid: {} {:?}", details, json))]
    JsonDeserializationInvalid { details: String, json: Value },
}

#[async_trait]
impl Remote for SingleNodeConnectionPool {
    type Conn = ElasticsearchStorage;

    /// Use the connection to create a client.
    async fn conn(self) -> Result<Self::Conn, RemoteError> {
        let transport = TransportBuilder::new(self)
            .build()
            .context(ElasticsearchTransportError)
            .map_err(|err| RemoteError::Connection {
                source: Box::new(err),
            })?;

        let response = transport
            .send::<String, String>(Method::Get, "/", HeaderMap::new(), None, None, None)
            .await
            .context(ElasticsearchConnectionError)
            .map_err(|err| RemoteError::Connection {
                source: Box::new(err),
            })?;

        if response.status_code().is_success() {
            // Response similar to:
            // {
            //   "index1": {
            //      "aliases": {
            //         "alias1": {},
            //         "alias2": {}
            //      }
            //   },
            //   "index2": {
            //      "aliases": {
            //         "alias3": {}
            //      }
            //   }
            // }
            let json = response
                .json::<Value>()
                .await
                .context(JsonDeserializationError)
                .map_err(|err| RemoteError::Connection {
                    source: Box::new(err),
                })?;
            let version_number = json
                .as_object()
                .ok_or(Error::JsonDeserializationInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })
                .map_err(|err| RemoteError::Connection {
                    source: Box::new(err),
                })?
                .get("version")
                .ok_or(Error::JsonDeserializationInvalid {
                    details: String::from("expected 'version'"),
                    json: json.clone(),
                })
                .map_err(|err| RemoteError::Connection {
                    source: Box::new(err),
                })?
                .as_object()
                .ok_or(Error::JsonDeserializationInvalid {
                    details: String::from("expected JSON object"),
                    json: json.clone(),
                })
                .map_err(|err| RemoteError::Connection {
                    source: Box::new(err),
                })?
                .get("number")
                .ok_or(Error::JsonDeserializationInvalid {
                    details: String::from("expected 'version.number'"),
                    json: json.clone(),
                })
                .map_err(|err| RemoteError::Connection {
                    source: Box::new(err),
                })?
                .as_str()
                .ok_or(Error::JsonDeserializationInvalid {
                    details: String::from("expected JSON string"),
                    json: json.clone(),
                })
                .map_err(|err| RemoteError::Connection {
                    source: Box::new(err),
                })?;
            let version = Version::parse(version_number).unwrap();
            lazy_static! {
                static ref VERSION_REQ: VersionReq = VersionReq::parse(ES_VERSION_REQ).unwrap();
            }
            if !VERSION_REQ.matches(&version) {
                Err(RemoteError::Connection {
                    source: Box::new(Error::ElasticsearchException {
                        msg: format!(
                            "Elasticsearch Invalid version: Expected '{}', got '{}'",
                            ES_VERSION_REQ, version
                        ),
                    }),
                })
            } else {
                let client = Elasticsearch::new(transport);
                Ok(ElasticsearchStorage::new(client))
            }
        } else {
            Err(RemoteError::Connection {
                source: Box::new(Error::ElasticsearchException {
                    msg: String::from("Elasticsearch Response Error"),
                }),
            })
        }
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
