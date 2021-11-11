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

#[cfg(test)]
mod tests {
    use super::*;
    use futures::TryStreamExt;
    use mimir::adapters::secondary::elasticsearch::{remote, ElasticsearchStorageConfig};
    use mimir::domain::ports::primary::list_documents::ListDocuments;
    use mimir::utils::docker;
    use mimirsbrunn::settings::poi2mimir as settings;
    use places::poi::Poi;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn should_correctly_index_poi_file() {
        docker::initialize()
            .await
            .expect("elasticsearch docker initialization");
        let opts = settings::Opts {
            config_dir: [env!("CARGO_MANIFEST_DIR"), "config"].iter().collect(),
            run_mode: Some("testing".to_string()),
            settings: vec![],
            input: [
                env!("CARGO_MANIFEST_DIR"),
                "tests",
                "fixtures",
                "poi",
                "keolis.poi",
            ]
            .iter()
            .collect(),
            cmd: settings::Command::Run,
        };

        let settings = settings::Settings::new(&opts).unwrap();
        let _res = mimirsbrunn::utils::launch::launch_async(move || run(opts, settings)).await;

        // Now we query the index we just created. Since it's a small cosmogony file with few entries,
        // we'll just list all the documents in the index, and check them.
        let config = ElasticsearchStorageConfig::default_testing();

        let client = remote::connection_pool_url(&config.url)
            .conn(config)
            .await
            .expect("Elasticsearch Connection Established");

        let pois: Vec<Poi> = client
            .list_documents()
            .await
            .unwrap()
            .try_collect()
            .await
            .unwrap();

        assert_eq!(pois.len(), 35);

        // let addr1 = addresses
        //     .iter()
        //     .find(|&addr| addr.name == "10 Place de la Mairie")
        //     .unwrap();

        // assert_eq!(addr1.id, "addr:1.378886;43.668175:10");
    }
}
