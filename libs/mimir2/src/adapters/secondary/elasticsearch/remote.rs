use async_trait::async_trait;
use elasticsearch::http::headers::HeaderMap;
use elasticsearch::http::transport::{
    BuildError as TransportBuilderError, SingleNodeConnectionPool, TransportBuilder,
};
use elasticsearch::http::Method;
use elasticsearch::Elasticsearch;
use semver::{Version, VersionReq};
use serde_json::Value;
use snafu::{ResultExt, Snafu};
use std::time::Duration;
use url::Url;

use super::ElasticsearchStorage;
use crate::domain::ports::secondary::remote::{Error as RemoteError, Remote};

pub const ES_KEY: &str = "ELASTICSEARCH_URL";
pub const ES_TEST_KEY: &str = "ELASTICSEARCH_TEST_URL";

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

    /// Invalid Version Requirements
    #[snafu(display("Invalid Version Requirement Specification {}: {}", details, source))]
    VersionRequirementInvalid {
        details: String,
        source: semver::Error,
    },
}

#[async_trait]
impl Remote for SingleNodeConnectionPool {
    type Conn = ElasticsearchStorage;

    /// Returns an Elasticsearch client
    ///
    /// This function verifies that the Elasticsearch server's version matches the requirements.
    ///
    /// # Arguments
    ///
    /// * `timeout` - Expressed in milliseconds.
    /// * `version_req` - Elasticsearch version requirements, eg '>=7.11.0'
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// // You can have rust code between fences inside the comments
    /// // If you pass --test to `rustdoc`, it will even test it for you!
    /// use mimir2::domain::ports::secondary::remote::Remote;
    /// use mimir2::adapters::secondary::elasticsearch;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///   let pool = elasticsearch::remote::connection_pool().await.unwrap();
    ///   let client = pool.conn(50u64, ">=7.10.0").await.unwrap();
    /// }
    ///
    /// ```
    async fn conn(self, timeout: u64, version_req: &str) -> Result<Self::Conn, RemoteError> {
        let version_req = VersionReq::parse(version_req)
            .context(VersionRequirementInvalid {
                details: version_req,
            })
            .map_err(|err| RemoteError::Connection {
                source: Box::new(err),
            })?;
        let transport = TransportBuilder::new(self)
            .build()
            .context(ElasticsearchTransportError)
            .map_err(|err| RemoteError::Connection {
                source: Box::new(err),
            })?;

        let timeout = Duration::from_millis(timeout);

        let response = transport
            .send::<String, String>(
                Method::Get,
                "/",
                HeaderMap::new(),
                None, /* query_string */
                None, /* body */
                Some(timeout),
            )
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
            if !version_req.matches(&version) {
                Err(RemoteError::Connection {
                    source: Box::new(Error::ElasticsearchException {
                        msg: format!(
                            "Elasticsearch Invalid version: Expected '{}', got '{}'",
                            version_req, version
                        ),
                    }),
                })
            } else {
                let client = Elasticsearch::new(transport);
                Ok(ElasticsearchStorage { client, timeout })
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
