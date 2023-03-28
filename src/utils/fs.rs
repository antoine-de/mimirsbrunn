use std::path::{Path, PathBuf};
use tracing::debug;

/// Walk over all files of an input path. If it is a directory all files from
/// this directory or subdirectory are yielded, if it is a file, it will be the
/// only yielded value.
pub fn walk_files_recursive(path: &Path) -> impl Iterator<Item = PathBuf> + 'static {
    let path = path.to_path_buf();
    walkdir::WalkDir::new(path.clone())
        .into_iter()
        .filter_map(move |res_dir_entry| match res_dir_entry {
            Ok(dir_entry) => Some(dir_entry),
            Err(err) => {
                debug!("failed to access a file when walking though directory '{path:?}': {err}");
                None
            }
        })
        .filter(|entry| entry.file_type().is_file())
        .map(|dir_entry| dir_entry.into_path())
}
