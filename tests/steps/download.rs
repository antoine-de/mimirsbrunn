use crate::error::Error;
use crate::utils::{create_dir_if_not_exists_rec, file_exists};
use crate::{error, State, Step, StepStatus};
use async_trait::async_trait;
use cucumber::{t, Steps};
use snafu::ResultExt;
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

const GEOFABRIK_URL: &str = "https://download.geofabrik.de";

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.given_regex_async(
        "osm file has been downloaded for (.*)",
        t!(|mut state, ctx| {
            let region = ctx.matches[1].clone();

            state
                .execute(DownloadOsm(region))
                .await
                .expect("failed to download OSM file");

            state
        }),
    );

    steps
}

async fn download_to_file(path: &Path, url: &str) -> Result<(), Error> {
    let mut file = tokio::io::BufWriter::new({
        fs::File::create(&path).await.context(error::InvalidIO {
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
    async fn execute(&mut self, _state: &State) -> Result<StepStatus, Error> {
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
