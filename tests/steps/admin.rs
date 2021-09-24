use common::document::ContainerDocument;
use config::Config;
use cucumber::{t, Steps};
use mimir2::{
    adapters::primary::common::{dsl::build_query, filters::Filters},
    adapters::secondary::elasticsearch::remote::{connection_test_pool, Error as PoolError},
    adapters::secondary::elasticsearch::{self, ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ},
    domain::model::query::Query,
    domain::ports::primary::search_documents::SearchDocuments,
    domain::ports::secondary::remote::Error as ConnectionError,
    domain::ports::secondary::remote::Remote,
    domain::ports::secondary::storage::Storage,
};
use places::admin::Admin;
use snafu::{ResultExt, Snafu};

use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use url::Url;

pub fn steps() -> Steps<crate::MyWorld> {
    let mut steps: Steps<crate::MyWorld> = Steps::new();

    steps.given_regex_async(
        "osm file has been downloaded for (.*)",
        t!(|mut world, ctx| {
            let region = ctx.matches[1].clone();
            world.processing_step = Some(download_osm(&region).await.unwrap());
            world
        }),
    );

    steps.given_regex_async(
        "osm file has been processed by cosmogony for (.*)",
        t!(|mut world, ctx| {
            let region = ctx.matches[1].clone();
            let previous = world.processing_step.clone().unwrap(); // we must have done something before, file either skipped or downloaded.
            world.processing_step = Some(generate_cosmogony(&region, previous).await.unwrap());
            world
        }),
    );

    steps.given_regex_async(
        "cosmogony file has been indexed for (.*)",
        t!(|mut world, ctx| {
            let region = ctx.matches[1].clone();
            let previous = world.processing_step.clone().unwrap(); // we must have done something before, file either skipped or downloaded.
            world.processing_step = Some(index_cosmogony(&region, previous).await.unwrap());
            world
        }),
    );

    steps.when_regex_async(
        "the user searches for \"(.*)\"",
        t!(|mut world, ctx| {
            let pool = elasticsearch::remote::connection_test_pool().await.unwrap();
            let client = pool
                .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
                .await
                .unwrap();
            let dsl = build_query(
                &ctx.matches[1],
                Filters::default(),
                &["fr"],
                &world.query_settings,
            );

            world.search_result = {
                client
                    .search_documents(
                        vec![Admin::static_doc_type().to_string()],
                        Query::QueryDSL(dsl),
                    )
                    .await
                    .unwrap()
            };

            world
        }),
    );

    steps.then_regex("he finds \"(.*)\" as the first result", |world, ctx| {
        assert_eq!(
            rank(&ctx.matches[1], &world.search_result)
                .expect("the user must perform a search before checking results"),
            0
        );

        world
    });

    steps
}

#[derive(PartialEq, Clone)]
pub enum ProcessingStep {
    Downloaded,
    Generated,
    Indexed,
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
}

// Given the name of a french region, it will download the matching OSM file
// If that file is already in the local filesystem, then we skip the download.
// This function makes several assumptions:
// 1. The name of the region is one found in http://download.geofabrik.de/europe/france.html
// 2. The file will be downloaded to `tests/data/osm` under the project's root (identified
//    by the CARGO_MANIFEST_DIR environment variable
// The function returns either `ProcessingStep::Skipped` or `ProcessingStep::Downloaded`,
// or an error.
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
    let url = Url::parse(&url).context(InvalidUrl { details: url })?;

    let resp = reqwest::get(url.clone()).await.context(Download {
        details: format!("could not download url {}", url),
    })?;

    // If we got a response which is an error (eg 404), then turn it
    // into an Error.
    let mut resp = resp.error_for_status().context(Download {
        details: format!("download response error {}", url),
    })?;

    let file = tokio::fs::File::create(&path).await.context(InvalidIO {
        details: format!("could no create file {}", path.display()),
    })?;

    // Do an asynchronous, buffered copy of the download to the output file
    let mut file = tokio::io::BufWriter::new(file);

    while let Some(chunk) = resp.chunk().await.context(Download {
        details: String::from("read chunk"),
    })? {
        file.write(&chunk).await.context(InvalidIO {
            details: String::from("write chunk"),
        })?;
    }

    file.flush().await.context(InvalidIO {
        details: String::from("flush"),
    })?;

    Ok(ProcessingStep::Downloaded)
}

// Generate a cosmogony file given the region
// The function makes several assumptions:
// 1. The OSM file has previously been downloaded into the expected folder (tests/data/osm)
// 2. The output is a jsonl.gz
// 3. The output will be stored in tests/data/cosmogony
//
// The OSM file will be processed if:
// 1. The output file is not found
// 2. If the output file is found and the previous step is 'downloaded' (that is it's probably a
//    new OSM file and we need to generate a new cosmogony file.
async fn generate_cosmogony(
    region: &str,
    previous: ProcessingStep,
) -> Result<ProcessingStep, Error> {
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
    let filename = format!("{}.jsonl.gz", region);
    output_path.push(&filename);
    // if the output already exists, and the input is not a new file, then skip the generation
    if (tokio::fs::metadata(&output_path).await.is_ok()) && (previous != ProcessingStep::Downloaded)
    {
        return Ok(ProcessingStep::Skipped);
    }
    let cosmogony_path = std::env::var("COSMOGONY_EXE").context(EnvironmentVariable {
        details: String::from("Could not get cosmogony executable"),
    })?;

    let mut child = tokio::process::Command::new(&cosmogony_path)
        .arg("--country-code")
        .arg("FR")
        .arg("--input")
        .arg(&input_path)
        .arg("--output")
        .arg(&output_path)
        .spawn()
        .expect("failed to spawn cosmogony");

    let _status = child.wait().await.context(InvalidIO {
        details: format!(
            "failed to generate cosmogony with input {} and output {}",
            input_path.display(),
            output_path.display()
        ),
    })?;

    // TODO Do something with the status?

    Ok(ProcessingStep::Generated)
}

async fn index_cosmogony(region: &str, previous: ProcessingStep) -> Result<ProcessingStep, Error> {
    let pool = connection_test_pool().await.context(ElasticsearchPool {
        details: String::from("Could not retrieve Elasticsearch test pool"),
    })?;

    let client = pool.conn().await.context(ElasticsearchConnection {
        details: String::from("Could not establish connection to Elasticsearch"),
    })?;

    let index = client
        .find_container(String::from("munin_admin"))
        .await
        .expect("Looking up munin_admin");

    // if the previous step is 'generated', then we need to index the cosmogony file.
    // Otherwise, we skip.
    // TODO: change this logic to check immutably what appends?
    if (previous != ProcessingStep::Generated) && (index.is_some()) {
        return Ok(ProcessingStep::Skipped);
    }
    let path: &'static str = env!("CARGO_MANIFEST_DIR");
    let mut input_path: PathBuf = [path, "tests", "data", "cosmogony"].iter().collect();
    let filename = format!("{}.jsonl.gz", region);
    input_path.push(&filename);
    mimirsbrunn::admin::index_cosmogony(
        input_path.into_os_string().into_string().unwrap(),
        vec![String::from("fr")],
        Config::builder()
            .add_source(Admin::default_es_container_config())
            .set_override("name", "test_admin")
            .expect("failed to set index name in config")
            .build()
            .expect("failed to build configuration"),
        client,
    )
    .await
    .map_err(|err| Error::Indexing {
        details: format!("could not index cosmogony: {}", err.to_string(),),
    })?;
    Ok(ProcessingStep::Indexed)
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
