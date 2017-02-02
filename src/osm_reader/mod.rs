extern crate osmpbfreader;
extern crate mimir;

use std::fs::File;
use std::path::Path;

pub mod osm_utils;
pub mod admin;
pub mod poi;
pub mod street;

pub type OsmPbfReader = osmpbfreader::OsmPbfReader<File>;

pub fn parse_osm_pbf(path: &str) -> OsmPbfReader {
    let path = Path::new(&path);
    osmpbfreader::OsmPbfReader::new(File::open(&path).unwrap())
}
