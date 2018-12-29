extern crate failure;
extern crate mimir;
extern crate osmpbfreader;

use std::fs::File;
use std::path::Path;
use crate::Error;

pub mod admin;
pub mod osm_utils;
pub mod poi;
pub mod street;

pub type OsmPbfReader = osmpbfreader::OsmPbfReader<File>;

pub fn make_osm_reader(path: &Path) -> Result<OsmPbfReader, Error> {
    Ok(osmpbfreader::OsmPbfReader::new(File::open(&path)?))
}
