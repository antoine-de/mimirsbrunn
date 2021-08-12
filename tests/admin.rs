use cucumber::async_trait;
use mimir2::adapters::primary::bragi::settings::QuerySettings;
use snafu::{ResultExt, Snafu};
use std::convert::Infallible;
use std::env;
use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use url::Url;

pub struct MyWorld {
    query_settings: QuerySettings,
    search_result: Vec<serde_json::Value>,
}

#[async_trait(?Send)]
impl cucumber::World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        let mut query_settings_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        query_settings_file.push("config/query_settings.toml");
        let query_settings = QuerySettings::new_from_file(query_settings_file)
            .await
            .expect("query settings");
        Ok(Self {
            query_settings,
            search_result: Vec::new(),
        })
    }
}

mod example_steps {
    use cucumber::{t, Steps};
    use log::*;
    // use failure::format_err;
    use super::download_osm;
    use mimir2::{
        adapters::primary::bragi::autocomplete::{build_query, Filters},
        adapters::secondary::elasticsearch,
        domain::ports::remote::Remote,
        domain::ports::search::SearchParameters,
        domain::usecases::search_documents::{SearchDocuments, SearchDocumentsParameters},
        domain::usecases::UseCase,
    };
    use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street, MimirObject};

    pub fn rank(id: &str, list: &[serde_json::Value]) -> Option<usize> {
        list.iter()
            .enumerate()
            // .find(|(_i, v)| v.as_object().unwrap().get("id").unwrap().as_str().unwrap() == id)
            .find(|(_i, v)| {
                let idr = v.as_object().unwrap().get("id").unwrap().as_str().unwrap();
                info!("id: {}", idr);
                idr == id
            })
            .map(|(i, _r)| i)
    }

    pub fn steps() -> Steps<crate::MyWorld> {
        let mut builder: Steps<crate::MyWorld> = Steps::new();

        builder
            .given_regex_async(
                "(.*) have been loaded using (.*) from (.*)",
                t!(|world, matches, _step| {
                    let _t = matches[1].clone();
                    let u = matches[2].clone();
                    download_osm(&u).await.unwrap();
                    world
                }),
            )
            .when_regex_async(
                "the user searches for \"(.*)\"",
                t!(|mut world, matches, _step| {
                    let pool = elasticsearch::remote::connection_test_pool().await.unwrap();

                    let client = pool.conn().await.unwrap();

                    let search_documents = SearchDocuments::new(Box::new(client));

                    let filters = Filters::default();

                    let query = build_query(&matches[1], filters, &["fr"], &world.query_settings);

                    info!("She pretty");

                    let parameters = SearchDocumentsParameters {
                        parameters: SearchParameters {
                            dsl: query,
                            doc_types: vec![
                                String::from(Admin::doc_type()),
                                String::from(Street::doc_type()),
                                String::from(Addr::doc_type()),
                                String::from(Stop::doc_type()),
                                String::from(Poi::doc_type()),
                            ],
                        },
                    };
                    world.search_result = search_documents.execute(parameters).await.unwrap();
                    world
                }),
            )
            .then_regex(
                r"^he finds (.*) in the first (.*) results.$",
                |world, matches, _step| {
                    let limit = matches[2].parse::<usize>().expect("limit");
                    let rank = rank(&matches[1], &world.search_result).unwrap();
                    assert!(rank < limit);
                    world
                },
            );

        builder
    }
}

#[tokio::main]
async fn main() {
    // Do any setup you need to do before running the Cucumber runner.
    // e.g. setup_some_db_thing()?;

    let _ = env_logger::builder().is_test(true).try_init();
    cucumber::Cucumber::<MyWorld>::new()
        // Specifies where our feature files exist
        .features(&["./features/admin"])
        // Adds the implementation of our steps to the runner
        .steps(example_steps::steps())
        // Parses the command line arguments if passed
        .cli()
        // Runs the Cucumber tests and then exists
        .run_and_exit()
        .await
}

#[derive(Debug, Snafu)]
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
    DownloadError {
        details: String,
        source: reqwest::Error,
    },
}

async fn download_osm(region: &str) -> Result<(), Error> {
    // Here we make sure that there is a folder tests/data/osm at the root of the project.
    let path: &'static str = env!("CARGO_MANIFEST_DIR");
    let mut path = PathBuf::from(path);
    path.push("tests");
    path.push("data");
    if tokio::fs::metadata(&path).await.is_err() {
        tokio::fs::create_dir(&path).await.context(InvalidIO {
            details: format!("could no create directory {}", path.display()),
        })?;
    }
    path.push("osm");
    if tokio::fs::metadata(&path).await.is_err() {
        tokio::fs::create_dir(&path).await.context(InvalidIO {
            details: format!("could no create directory {}", path.display()),
        })?;
    }
    // Then we try to see if there is already a file with the expected name in that
    // folder, in which case we skip the actual download, to save time.
    let filename = format!("{}-latest.osm.pbf", region);
    path.push(&filename);
    if tokio::fs::metadata(&path).await.is_ok() {
        return Ok(());
    }

    // Ok, directory structure is fine, no file in 'cache', so we download
    let url = format!(
        "https://download.geofabrik.de/europe/france/{}-latest.osm.pbf",
        region
    );
    let url = Url::parse(&url).context(InvalidUrl {
        details: String::from(url),
    })?;
    let body = reqwest::get(url.clone())
        .await
        .context(DownloadError {
            details: format!("could not download url {}", url),
        })?
        .text()
        .await
        .context(DownloadError {
            details: format!("could not decode response body for url {}", url),
        })?;
    let mut file = tokio::fs::File::create(&path).await.context(InvalidIO {
        details: format!("could no create file {}", path.display()),
    })?;

    file.write(body.as_bytes()).await.context(InvalidIO {
        details: format!("could no write to file {}", path.display()),
    })?;

    Ok(())
}
