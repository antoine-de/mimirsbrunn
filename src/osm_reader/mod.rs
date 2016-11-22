extern crate osmpbfreader;
extern crate mimir;

use std::fs::File;
use std::rc::Rc;
use std::path::Path;

pub mod utils;
pub mod osm_admin_reader;
pub mod osm_poi_reader;
pub mod osm_street_reader;

pub type AdminsVec = Vec<Rc<mimir::Admin>>;
pub type OsmPbfReader = osmpbfreader::OsmPbfReader<File>;

pub fn parse_osm_pbf(path: &str) -> OsmPbfReader {
    let path = Path::new(&path);
    osmpbfreader::OsmPbfReader::new(File::open(&path).unwrap())
}
