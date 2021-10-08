use async_trait::async_trait;
use cucumber::{t, StepContext, Steps};
use futures::stream::{self, TryStreamExt};
use snafu::ResultExt;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

use crate::error;
use crate::error::Error;
use crate::state::{State, Step, StepStatus};
use crate::utils::{create_dir_if_not_exists_rec, file_exists};

const GEOFABRIK_URL: &str = "https://download.geofabrik.de";
const BANO_URL: &str = "http://bano.openstreetmap.fr";

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.given_regex_async(
        "osm file has been downloaded for (.*)",
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();

            state
                .execute_once(DownloadOsm(region), &ctx)
                .await
                .expect("failed to download OSM file");

            state
        }),
    );

    steps.given_regex_async(
        "bano files have been downloaded for (.*) into (.*)",
        t!(|mut state, ctx| {
            let departments = ctx.matches[1]
                .split(',')
                .map(|s| s.trim().to_string())
                .collect();

            let region = ctx.matches[2].clone();

            state
                .execute_once(
                    DownloadBano {
                        departments,
                        region,
                    },
                    &ctx,
                )
                .await
                .expect("failed to download OSM file");

            state
        }),
    );

    steps
}

/// Downloads the file identified by the url and saves it to the given path.
/// If a file is already present, it will append to that file.
async fn download_to_file(path: &Path, url: &str) -> Result<(), Error> {
    let mut file = tokio::io::BufWriter::new({
        fs::OpenOptions::new()
            .append(true)
            .create(true)
            .open(&path)
            .await
            .context(error::InvalidIO {
                details: format!("could no create file for download {}", path.display()),
            })?
    });

    let mut resp = reqwest::get(url)
        .await
        .context(error::Download {
            details: format!("could not download url {}", url),
        })?
        .error_for_status()
        .context(error::Download {
            details: format!("download response error for {}", url),
        })?;

    while let Some(chunk) = resp.chunk().await.context(error::Download {
        details: format!("read chunk error during download of {}", url),
    })? {
        file.write_all(&chunk).await.context(error::InvalidIO {
            details: format!("write chunk error during download of {}", url),
        })?;
    }

    file.flush().await.context(error::InvalidIO {
        details: format!("flush error during download of {}", url),
    })?;

    Ok(())
}

/// Given the name of a french region, it will download the matching OSM file
/// If that file is already in the local filesystem, then we skip the download.
///
/// This makes several assumptions:
///  1. The name of the region is one found in http://download.geofabrik.de/europe/france.html
///  2. The file will be downloaded to `tests/data/osm` under the project's root (identified
///     by the CARGO_MANIFEST_DIR environment variable
#[derive(PartialEq)]
pub struct DownloadOsm(pub String);

#[async_trait(?Send)]
impl Step for DownloadOsm {
    async fn execute(&mut self, _state: &State, _ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self(region) = self;

        // Try to see if there is already a file with the expected name in tests/data/osm, in which
        // case we skip the actual download, to save time.
        let dir_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "data", "osm"]
            .iter()
            .collect();

        let file_path = dir_path.join(format!("{}-latest.osm.pbf", region));

        if file_exists(&file_path).await {
            return Ok(StepStatus::Skipped);
        }

        // No file in 'cache', so we download it
        create_dir_if_not_exists_rec(&dir_path).await?;
        let url = format!("{}/europe/france/{}-latest.osm.pbf", GEOFABRIK_URL, region);
        download_to_file(&file_path, &url).await?;

        Ok(StepStatus::Done)
    }
}

/// Given a list of French departments, it will download the matching BANO files.
/// If these files are already in the local filesystem, then we skip the download.
/// Then we concatenate these files into a single file with the name of the region.
/// The reason for this is that we want at the indexing stage to check that admins
/// have been indexed prior to indexing addresses, and so we need the same name for
/// the bano region and the osm region.
///
/// This makes several assumptions:
///  1. The file will be downloaded to `tests/data/bano` under the project's root (identified
///     by the CARGO_MANIFEST_DIR environment variable
#[derive(PartialEq)]
pub struct DownloadBano {
    pub departments: Vec<String>,
    pub region: String,
}

#[async_trait(?Send)]
impl Step for DownloadBano {
    // FIXME: The only StepStatus returned is StepStatus::Done. Not handling very well if there
    // are files downloaded before.
    // This function will not stop if one of the download fails, but it will report an error.
    async fn execute(&mut self, _state: &State, _ctx: &StepContext) -> Result<StepStatus, Error> {
        let Self {
            departments,
            region,
        } = self;
        let dir_path: PathBuf = [env!("CARGO_MANIFEST_DIR"), "tests", "data", "bano"]
            .iter()
            .collect();

        create_dir_if_not_exists_rec(&dir_path).await?;

        // this is the path for the concatenated departments
        let file_path = &dir_path.join(format!("{}.csv", region));

        if file_exists(file_path).await {
            return Ok(StepStatus::Skipped);
        }

        stream::iter(departments.iter().map(Ok))
            .try_for_each(|department| async move {
                let url = format!("{}/data/bano-{:02}.csv", BANO_URL, department);
                download_to_file(file_path, &url).await
            })
            .await?;

        Ok(StepStatus::Done)
    }
}
