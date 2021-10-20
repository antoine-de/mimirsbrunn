use async_trait::async_trait;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Connection Error: {}", source))]
    Connection { source: Box<dyn std::error::Error> },
}

#[async_trait]
pub trait Remote {
    type Conn;
    type Config;

    /// Returns a client for making calls to the backend
    ///
    /// This function verifies that the backend's version matches the requirements.
    ///
    /// # Arguments
    ///
    /// * `config`  - Elasticsearch configuration. See config/elasticsearch/default.toml
    ///
    /// # Examples
    ///
    /// The following example creates a connection pool, and then uses that connection pool to
    /// create a client for Elasticsearch, making sure that the version is greater than 7.11.0
    ///
    /// ```rust,no_run
    /// use url::Url;
    /// use mimir2::domain::ports::secondary::remote::Remote;
    /// use mimir2::adapters::secondary::elasticsearch;
    /// use mimir2::adapters::secondary::elasticsearch::ElasticsearchStorageConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///   let url = Url::parse("http://localhost:9200").expect("valid url");
    ///   let client = elasticsearch::remote::connection_pool_url(&url)
    ///       .conn(ElasticsearchStorageConfig::default_testing()).await.unwrap();
    /// }
    ///
    /// ```
    async fn conn(self, config: Self::Config) -> Result<Self::Conn, Error>;
}
