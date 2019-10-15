use git_version;
use json;

use std::ffi::OsStr;
use std::fs::{self, File};
use std::io::{self, BufReader, Error, ErrorKind, Read};
use std::path::Path;

fn validate_json_files(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            validate_json_files(&path)?;
        } else if path.extension() == Some(OsStr::new("json")) {
            let mut reader = BufReader::new(File::open(&path)?);
            let mut content = String::new();
            reader.read_to_string(&mut content)?;

            json::parse(&content).map_err(|e| {
                Error::new(
                    ErrorKind::InvalidData,
                    format!("{}: invalid json: {}", path.display(), e),
                )
            })?;
        }
    }
    Ok(())
}

fn main() {
    if let Err(e) = validate_json_files(Path::new("json")) {
        eprintln!("=> Failure in JSON validation!\n=> {}", e);
        panic!("");
    }
    git_version::set_env_with_name("CARGO_PKG_VERSION");
}
