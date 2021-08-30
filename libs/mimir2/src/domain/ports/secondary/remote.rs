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
    async fn conn(self) -> Result<Self::Conn, Error>;
}
