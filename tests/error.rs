use mimir2::adapters::secondary::elasticsearch::remote::Error as PoolError;
use mimir2::domain::ports::secondary::remote::Error as ConnectionError;
use snafu::Snafu;

#[derive(Debug, Snafu)]
#[snafu(visibility = "pub(crate)")]
pub enum Error {
    #[snafu(display("Invalid Download URL: {} ({})", source, details))]
    InvalidUrl {
        details: String,
        source: url::ParseError,
    },
    #[snafu(display("Invalid IO: {} ({})", source, details))]
    InvalidIO {
        details: String,
        source: std::io::Error,
    },
    #[snafu(display("Download Error: {} ({})", source, details))]
    Download {
        details: String,
        source: reqwest::Error,
    },
    #[snafu(display("Elasticsearch Pool Error: {} ({})", source, details))]
    ElasticsearchPool { details: String, source: PoolError },
    #[snafu(display("Elasticsearch Connection Error: {} ({})", source, details))]
    ElasticsearchConnection {
        details: String,
        source: ConnectionError,
    },
    #[snafu(display("Indexing Error: {}", details))]
    Indexing { details: String },

    #[snafu(display("JSON Error: {} ({})", details, source))]
    Json {
        details: String,
        source: serde_json::Error,
    },

    #[snafu(display("Environment Variable Error: {} ({})", details, source))]
    EnvironmentVariable {
        details: String,
        source: std::env::VarError,
    },

    #[snafu(display("Miscellaneous Error: {}", details))]
    Miscellaneous { details: String },
}
