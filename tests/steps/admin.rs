use cucumber::{t, Steps};
use mimir2::{
    adapters::primary::bragi::autocomplete::{build_query, Filters},
    adapters::secondary::elasticsearch,
    domain::ports::remote::Remote,
    domain::ports::search::SearchParameters,
    domain::usecases::search_documents::{SearchDocuments, SearchDocumentsParameters},
    domain::usecases::UseCase,
};
use mimir2::{
    adapters::secondary::elasticsearch::{
        internal::{IndexConfiguration, IndexMappings, IndexParameters, IndexSettings},
        remote::{connection_test_pool, Error as PoolError},
    },
    domain::ports::remote::Error as ConnectionError,
};
use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street, MimirObject};
use snafu::{ResultExt, Snafu};

use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use url::Url;

pub fn steps() -> Steps<crate::MyWorld> {
    let mut steps: Steps<crate::MyWorld> = Steps::new();

    steps.given_regex_async(
        "osm file has been downloaded for (.*)",
        t!(|world, ctx| {
            let region = ctx.matches[1].clone();
            download_osm(&region).await.unwrap();
            world
        }),
    );

    steps.given_regex_async(
        "osm file has been processed by cosmogony for (.*)",
        t!(|world, ctx| {
            let region = ctx.matches[1].clone();
            generate_cosmogony(&region).await.unwrap();
            index_cosmogony(&region).await.unwrap();
            world
        }),
    );

    steps.given_regex_async(
        "cosmogony file has been indexed for (.*)",
        t!(|world, ctx| {
            let region = ctx.matches[1].clone();
            index_cosmogony(&region).await.unwrap();
            world
        }),
    );

    steps.when_regex_async(
        "the user searches for \"(.*)\"",
        t!(|mut world, ctx| {
            let pool = elasticsearch::remote::connection_test_pool().await.unwrap();

            let client = pool.conn().await.unwrap();

            let search_documents = SearchDocuments::new(Box::new(client));

            let filters = Filters::default();

            let query = build_query(&ctx.matches[1], filters, &["fr"], &world.query_settings);

            let parameters = SearchDocumentsParameters {
                parameters: SearchParameters {
                    dsl: query,
                    doc_types: vec![String::from(Admin::doc_type())],
                },
            };
            world.search_result = search_documents.execute(parameters).await.unwrap();
            world
        }),
    );

    steps.then_regex(
        r"^he finds (.*) in the first (.*) results.$",
        |world, ctx| {
            let limit = ctx.matches[2].parse::<usize>().expect("limit");
            panic!("search results: {}", world.search_result[0]);
            let rank = rank(&ctx.matches[1], &world.search_result).unwrap();
            assert!(rank < limit);
            world
        },
    );

    steps
}

pub enum ProcessingStep {
    Done,
    Skipped,
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
    #[snafu(display("Elasticsearch Pool Error: {} ({})", source, details))]
    ElasticsearchPoolError { details: String, source: PoolError },
    #[snafu(display("Elasticsearch Connection Error: {} ({})", source, details))]
    ElasticsearchConnectionError {
        details: String,
        source: ConnectionError,
    },
    #[snafu(display("Indexing Error: {}", details))]
    IndexingError { details: String },

    #[snafu(display("JSON Error: {} ({})", details, source))]
    JsonError {
        details: String,
        source: serde_json::Error,
    },
}

async fn download_osm(region: &str) -> Result<ProcessingStep, Error> {
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
        return Ok(ProcessingStep::Skipped);
    }

    // Ok, directory structure is fine, no file in 'cache', so we download
    let url = format!(
        "https://download.geofabrik.de/europe/france/{}-latest.osm.pbf",
        region
    );
    let url = Url::parse(&url).context(InvalidUrl {
        details: String::from(url),
    })?;

    let resp = reqwest::get(url.clone()).await.context(DownloadError {
        details: format!("could not download url {}", url),
    })?;

    // If we got a response which is an error (eg 404), then turn it
    // into an Error.
    let mut resp = resp.error_for_status().context(DownloadError {
        details: format!("download response error {}", url),
    })?;

    let file = tokio::fs::File::create(&path).await.context(InvalidIO {
        details: format!("could no create file {}", path.display()),
    })?;

    // Do an asynchronous, buffered copy of the download to the output file
    let mut file = tokio::io::BufWriter::new(file);

    while let Some(chunk) = resp.chunk().await.context(DownloadError {
        details: String::from("read chunk"),
    })? {
        file.write(&chunk).await.context(InvalidIO {
            details: String::from("write chunk"),
        })?;
    }

    file.flush().await.context(InvalidIO {
        details: String::from("flush"),
    })?;

    Ok(ProcessingStep::Done)
}

// Generate a cosmogony file given the region (assuming the osm file
// has been previously downloaded. If a cosmogony file is already there,
// we skip the generation.
async fn generate_cosmogony(region: &str) -> Result<ProcessingStep, Error> {
    let path: &'static str = env!("CARGO_MANIFEST_DIR");
    let mut input_path: PathBuf = [path, "tests", "data", "osm"].iter().collect();
    let filename = format!("{}-latest.osm.pbf", region);
    input_path.push(&filename);
    let mut output_path: PathBuf = [path, "tests", "data", "cosmogony"].iter().collect();
    if tokio::fs::metadata(&output_path).await.is_err() {
        tokio::fs::create_dir(&output_path)
            .await
            .context(InvalidIO {
                details: format!("could no create directory {}", output_path.display()),
            })?;
    }
    let filename = format!("{}.json.gz", region);
    output_path.push(&filename);
    // if the output already exists, then skip the generation
    if tokio::fs::metadata(&output_path).await.is_ok() {
        return Ok(ProcessingStep::Skipped);
    }
    let mut child = tokio::process::Command::new(
        "/home/matt/lab/rust/kisio/cosmogony/target/release/cosmogony",
    )
    .arg("--country-code")
    .arg("FR")
    .arg("--input")
    .arg(&input_path)
    .arg("--output")
    .arg(&output_path)
    .spawn()
    .expect("failed to spawn cosmogony");

    let status = child.wait().await.context(InvalidIO {
        details: format!(
            "failed to generate cosmogony with input {} and output {}",
            input_path.display(),
            output_path.display()
        ),
    })?;

    // TODO Do something with the status?

    Ok(ProcessingStep::Done)
}

async fn index_cosmogony(region: &str) -> Result<(), Error> {
    let pool = connection_test_pool()
        .await
        .context(ElasticsearchPoolError {
            details: String::from("Could not retrieve Elasticsearch test pool"),
        })?;

    let client = pool.conn().await.context(ElasticsearchConnectionError {
        details: String::from("Could not establish connection to Elasticsearch"),
    })?;

    let path: &'static str = env!("CARGO_MANIFEST_DIR");
    let config_path: PathBuf = [path, "config", "admin"].iter().collect();
    let mut settings_path = config_path.clone();
    settings_path.push("settings.json");
    let settings = tokio::fs::read_to_string(&settings_path)
        .await
        .context(InvalidIO {
            details: format!(
                "could not read settings file from '{}'",
                settings_path.display()
            ),
        })?;

    let settings = serde_json::from_str(&settings).context(JsonError {
        details: String::from("Could not deserialize settings"),
    })?;

    let mut mappings_path = config_path.clone();
    mappings_path.push("mappings.json");
    let mappings = tokio::fs::read_to_string(&mappings_path)
        .await
        .context(InvalidIO {
            details: format!(
                "could not read mappings file from '{}'",
                mappings_path.display()
            ),
        })?;

    let mappings = serde_json::from_str(&mappings).context(JsonError {
        details: String::from("Could not deserialize settings"),
    })?;

    let config = IndexConfiguration {
        name: String::from("test"),
        parameters: IndexParameters {
            timeout: String::from("10s"),
            wait_for_active_shards: String::from("1"), // only the primary shard
        },
        settings: IndexSettings { value: settings },
        mappings: IndexMappings { value: mappings },
    };

    let mut input_path: PathBuf = [path, "tests", "data", "cosmogony"].iter().collect();
    let filename = format!("{}.json.gz", region);
    input_path.push(&filename);
    mimirsbrunn::admin::index_cosmogony(
        input_path.into_os_string().into_string().unwrap(),
        vec![String::from("fr")],
        config,
        client,
    )
    .await
    .map_err(|err| Error::IndexingError {
        details: format!("could not index cosmogony: {}", err.to_string(),),
    })
}

pub fn rank(id: &str, list: &[serde_json::Value]) -> Option<usize> {
    list.iter()
        .enumerate()
        // .find(|(_i, v)| v.as_object().unwrap().get("id").unwrap().as_str().unwrap() == id)
        .find(|(_i, v)| {
            let idr = v.as_object().unwrap().get("id").unwrap().as_str().unwrap();
            idr == id
        })
        .map(|(i, _r)| i)
}
