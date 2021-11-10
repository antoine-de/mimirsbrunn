use snafu::{ResultExt, Snafu};
use structopt::StructOpt;

use mimir::adapters::secondary::elasticsearch;
use mimir::domain::ports::secondary::remote::Remote;
use mimirsbrunn::settings::poi2mimir as settings;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Settings (Configuration or CLI) Error: {}", source))]
    Settings { source: settings::Error },

    #[snafu(display("Elasticsearch Connection Pool {}", source))]
    ElasticsearchConnection {
        source: mimir::domain::ports::secondary::remote::Error,
    },

    #[snafu(display("Configuration Error {}", source))]
    Configuration { source: common::config::Error },

    #[snafu(display("Execution Error {}", source))]
    Execution { source: Box<dyn std::error::Error> },
}

#[tokio::main]
async fn main() -> Result<(), Error> {
    let opts = settings::Opts::from_args();

    let settings = settings::Settings::new(&opts).context(Settings)?;

    match opts.cmd {
        settings::Command::Run => mimirsbrunn::utils::launch::wrapped_launch_async(
            &settings.logging.path.clone(),
            move || run(opts, settings),
        )
        .await
        .context(Execution),
        settings::Command::Config => {
            println!("{}", serde_json::to_string_pretty(&settings).unwrap());
            Ok(())
        }
    }
}

async fn run(
    opts: settings::Opts,
    settings: settings::Settings,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = elasticsearch::remote::connection_pool_url(&settings.elasticsearch.url)
        .conn(settings.elasticsearch.clone())
        .await
        .context(ElasticsearchConnection)?;

    mimirsbrunn::pois::index_pois(opts.input, &client, settings.container).await?;

    Ok(())
}
