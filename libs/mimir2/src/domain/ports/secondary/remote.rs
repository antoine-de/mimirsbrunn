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
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///   let pool = elasticsearch::remote::connection_pool().await.unwrap();
    ///   let client = pool.conn(50u64, ">=7.11.0").await.unwrap();
    /// }
    ///
    /// ```
    async fn conn(self, timeout: u64, version_req: &str) -> Result<Self::Conn, Error>;
}
