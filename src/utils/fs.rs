use std::path::{Path, PathBuf};
use tracing::warn;

/// Walk over all files of an input path. If it is a directory all files from
/// this directory or subdirectory are yielded, if it is a file, it will be the
/// only yielded value.
pub fn walk_files_recursive(path: &Path) -> impl Iterator<Item = PathBuf> + 'static {
    let path = path.to_path_buf();
    walkdir::WalkDir::new(path.clone())
        .into_iter()
        .filter_entry(|entry| entry.file_type().is_file())
        .filter_map(move |res_dir_entry| match res_dir_entry {
            Ok(dir_entry) => Some(dir_entry.into_path()),
            Err(err) => {
                warn!("failed to read file when walking through directory '{path:?}': {err}",);
                None
            }
        })
}
