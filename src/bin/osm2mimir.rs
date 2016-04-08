#[macro_use]
extern crate log;
extern crate env_logger;
extern crate osmpbfreader;
extern crate rustc_serialize;
extern crate docopt;
extern crate mimirsbrunn;

use std::collections::HashSet;
use std::collections::BTreeMap;
use osmpbfreader::{OsmObj, OsmId};

pub type AdminsMap = BTreeMap<OsmId, mimirsbrunn::Admin>;

#[derive(RustcDecodable, Debug)]
struct Args {
    flag_input: String,
    flag_level: Vec<u32>,
    flag_connection_string: String,
}

static USAGE: &'static str = "
Usage:
    osm2mimir --help
    osm2mimir --input=<file> [--connection-string=<connection-string>] --level=<level>...

Options:
    -h, --help            Show this message.
    -i, --input=<file>    OSM PBF file.
    -l, --level=<level>   Admin levels to keep.
    -c, --connection-string=<connection-string>
                          Elasticsearch parameters, [default: http://localhost:9200/munin]
";

fn get_osm_id(obj: &OsmObj) -> OsmId {
    match *obj {
        OsmObj::Node(ref node) => OsmId::Node(node.id),
        OsmObj::Way(ref way) => OsmId::Way(way.id),
        OsmObj::Relation(ref rel) => OsmId::Relation(rel.id),
    }
}

fn update_coordinates(filename: &String, admins: &mut AdminsMap) {
    if admins.is_empty() {
        return;
    }
    // load coord for administratives regions
    let path = std::path::Path::new(&filename);
    let r = std::fs::File::open(&path).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);
    for obj in pbf.iter() {
        if let osmpbfreader::OsmObj::Node(ref node) = obj {
            let mut adm = match admins.get_mut(&get_osm_id(&obj)) {
                Some(val) => val,
                None => continue,
            };
            adm.coord.lat = node.lat;
            adm.coord.lon = node.lon;
        }
    }
}

fn administartive_regions(filename: &String, levels: &HashSet<u32>) -> AdminsMap {
    let mut administrative_regions = AdminsMap::new();
    let path = std::path::Path::new(&filename);
    let r = std::fs::File::open(&path).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);
    // load administratives regions
    for obj in pbf.iter() {
        if let osmpbfreader::OsmObj::Relation(relation) = obj {
            // not administartive region
            if !relation.tags
                        .get("boundary")
                        .map(|s| s == "administrative")
                        .unwrap_or(false) {
                continue;
            }
            // admininstrative region without coordinates
            let admin_centre = match relation.refs.iter().find(|rf| rf.role == "admin_centre") {
                Some(val) => val.member,
                None => {
                    info!("adminstrative regione without coordinates for relation {}.",
                          relation.id);
                    continue;
                }
            };
            let level = relation.tags
                                .get("admin_level")
                                .and_then(|s| s.parse().ok());
            let level = match level {
                None => {
                    info!("invalid admin_level for relation {}: admin_level {:?}",
                          relation.id,
                          relation.tags.get("admin_level"));
                    continue;
                }
                Some(ref l) if !levels.contains(&l) => continue,
                Some(l) => l,
            };
            // administrative region with name ?
            let name = match relation.tags.get("name") {
                Some(val) => val,
                None => {
                    info!("adminstrative region without name for relation {}:  admin_level {} \
                           ignored.",
                          relation.id,
                          level);
                    continue;
                }
            };
            let admin_id = match relation.tags.get("ref:INSEE") {
                Some(val) => format!("admin:fr:{}", val),
                None => format!("admin:osm:{}", relation.id),
            };
            let zip_code = match relation.tags.get("addr:postcode") {
                Some(val) => &val[..],
                None => "",
            };
            let admin = mimirsbrunn::Admin {
                id: admin_id,
                level: level,
                name: name.to_string(),
                zip_code: zip_code.to_string(),
                // TODO weight value ?
                weight: 1,
                coord: mimirsbrunn::Coord {
                    lat: 0.0,
                    lon: 0.0,
                },
            };
            administrative_regions.insert(admin_centre, admin);
        }
    }
    return administrative_regions;
}

fn main() {
    env_logger::init().unwrap();
    debug!("importing adminstrative region into Mimir");
    let args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());
    let levels = args.flag_level.iter().cloned().collect();
    let mut res = administartive_regions(&args.flag_input, &levels);
    update_coordinates(&args.flag_input, &mut res);
}
