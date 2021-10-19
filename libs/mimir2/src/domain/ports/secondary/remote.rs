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
    /// * `timeout` - Expressed in milliseconds. This is used for establishing the connection to
    ///   the server, and on subsequent calls by the client to the server.
    /// * `version_req` - Backend version requirements, eg '>=7.11.0'
    ///
    /// # Examples
    ///
    /// The following example creates a connection pool, and then uses that connection pool to
    /// create a client for Elasticsearch, making sure that the version is greater than 7.11.0
    ///
    /// ```rust,no_run
    /// use mimir2::domain::ports::secondary::remote::Remote;
    /// use mimir2::adapters::secondary::elasticsearch;
    /// use mimir2::adapters::secondary::elasticsearch::ElasticsearchStorageConfig;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///   let url = "http://localhost:9200";
    ///   let pool = elasticsearch::remote::connection_pool_url(url).await.unwrap();
    ///   let client = pool.conn(ElasticsearchStorageConfig::default_testing()).await.unwrap();
    /// }
    ///
    /// ```
    async fn conn(self, config: Self::Config) -> Result<Self::Conn, Error>;
}
