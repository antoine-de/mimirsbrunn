use async_trait::async_trait;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Use Case Execution Error: {}", source))]
    #[snafu(visibility(pub))]
    Execution { source: Box<dyn std::error::Error> },
}

#[async_trait]
pub trait UseCase {
    type Res;
    type Param;

    async fn execute(&self, param: Self::Param) -> Result<Self::Res, Error>;
}

pub mod explain_query;
pub mod generate_index;
pub mod list_documents;
pub mod search_documents;
