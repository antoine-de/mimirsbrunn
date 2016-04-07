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
}



static USAGE: &'static str = "
Usage:
    osm2mimir --input=<pbf-file> --level=<admin-level>...

Options:
    -h, --help                               Show this message.
    -i <pbf-file>, --input=<pbf-file>        OSM PBF file.
    -l <admin-level>, --level=<admin-level>  Admin levels to keep.
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
                None => continue,

            };
            let level = relation.tags
                                .get("admin_level")
                                .and_then(|s| s.parse().ok())
                                .unwrap_or(0);
            // administrative region without levelval
            if !levels.contains(&level) {
                continue;
            }
            // administrative region with name ?
            let name = match relation.tags.get("name") {
                Some(val) => val,
                None => continue,
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
    println!("importing adminstrative region into Mimir");
    let args: Args = docopt::Docopt::new(USAGE)
                         .and_then(|d| d.decode())
                         .unwrap_or_else(|e| e.exit());
    let map = args.flag_level.iter().cloned().collect();
    let mut res = administartive_regions(&args.flag_input, &map);
    update_coordinates(&args.flag_input, &mut res);
}
