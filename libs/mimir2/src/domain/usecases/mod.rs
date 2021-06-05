use async_trait::async_trait;
use snafu::Snafu;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Use Case Execution Error: {}", details))]
    #[snafu(visibility(pub))]
    Execution { details: String },
}

#[async_trait]
pub trait UseCase {
    type Res;
    type Param;

    async fn execute(&self, param: Self::Param) -> Result<Self::Res, Error>;
}

pub mod generate_index;
