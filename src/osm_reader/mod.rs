extern crate mimir;
extern crate osmpbfreader;

use std::fs::File;
use std::path::Path;

pub mod admin;
pub mod osm_utils;
pub mod poi;
pub mod street;

pub type OsmPbfReader = osmpbfreader::OsmPbfReader<File>;

pub fn parse_osm_pbf(path: &Path) -> OsmPbfReader {
    osmpbfreader::OsmPbfReader::new(File::open(&path).unwrap())
}
