use clap::Parser;
use mimir::adapters::primary::templates;
use mimir::adapters::secondary::elasticsearch;
use mimir::domain::ports::secondary::remote::Remote;
use mimirsbrunn::settings::ctlmimir as settings;
use snafu::{ResultExt, Snafu};
use std::path::PathBuf;

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Settings (Configuration or CLI) Error: {}", source))]
    Settings { source: settings::Error },

    #[snafu(display("Elasticsearch Connection Pool {}", source))]
    ElasticsearchConnection {
        source: mimir::domain::ports::secondary::remote::Error,
    },

    #[snafu(display("Execution Error {}", source))]
    Execution { source: Box<dyn std::error::Error> },
}

fn main() -> Result<(), Error> {
    let opts = settings::Opts::parse();
    let settings = settings::Settings::new(&opts).context(Settings)?;

    match opts.cmd {
        settings::Command::Run => mimirsbrunn::utils::launch::launch_with_runtime(
            &settings.logging.path.clone(),
            settings.nb_threads,
            run(opts, settings),
        )
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

    tracing::info!("Trying to connect to elasticsearch at {}", &settings.elasticsearch.url);
    let client = elasticsearch::remote::connection_pool_url(&settings.elasticsearch.url)
        .conn(settings.elasticsearch)
        .await
        .context(ElasticsearchConnection)
        .map_err(Box::new)?;

    tracing::info!("Connected to elasticsearch.");

    let path: PathBuf = opts.config_dir
        .join("elasticsearch")
        .join("templates")
        .join("components");

    tracing::info!("Beginning components imports from {:?}", &path);
    templates::import(client.clone(), path, templates::Template::Component)
        .await
        .map_err(Box::new)?;

    let path: PathBuf = opts.config_dir
        .join("elasticsearch")
        .join("templates")
        .join("indices");

    tracing::info!("Beginning indices imports from {:?}", &path);
    templates::import(client, path, templates::Template::Index)
        .await
        .map_err(Box::new)?;

    Ok(())
}
