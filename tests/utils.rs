use crate::error;
use crate::error::Error;
use snafu::ResultExt;
use std::path::{Path, PathBuf};
use tokio::fs;

pub async fn file_exists(path: &Path) -> bool {
    fs::metadata(path).await.is_ok()
}

pub async fn create_dir_if_not_exists(path: &Path) -> Result<(), Error> {
    if !file_exists(path).await {
        fs::create_dir(path).await.context(error::InvalidIO {
            details: format!("could no create directory {}", path.display()),
        })?;
    }

    Ok(())
}

pub async fn create_dir_if_not_exists_rec(path: &Path) -> Result<(), Error> {
    let mut head = PathBuf::new();

    for fragment in path {
        head.push(fragment);
        create_dir_if_not_exists(&head).await?;
    }

    Ok(())
}
