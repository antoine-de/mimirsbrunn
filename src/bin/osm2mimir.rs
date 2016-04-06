extern crate osmpbfreader;
extern crate rustc_serialize;
extern crate docopt;
extern crate mimirsbrunn;

use std::collections::HashSet;

#[derive(RustcDecodable, Debug)]
struct Args {
    flag_input: String,
    flag_admin: Vec<u32>
}

static USAGE: &'static str = "
Usage:
    osm2mimir --input=<pbf-file> --admin=<admin-level>...
    
Options:
    -h, --help                               Show this message.
    -i <pbf-file>, --input=<pbf-file>        OSM PBF file.
    -a <admin-level>, --admin=<admin-level>  Admin levels to keep.
";

fn administartive_regions(filename: &String, levels: &HashSet<u32>) -> Vec<mimirsbrunn::Admin> {
	let mut administrative_regions = Vec::new();
    let path = std::path::Path::new(&filename);
    let r = std::fs::File::open(&path).unwrap();
    let mut pbf = osmpbfreader::OsmPbfReader::new(r);
    for obj in pbf.iter() {
		if let osmpbfreader::OsmObj::Relation(relation) = obj {
			let is_admin = relation.tags.get("boundary")
						   .map(|s| s == "administrative")
						   .unwrap_or(false);
			if ! is_admin {
				continue;
			}
			let level = relation.tags.get("admin_level")
						.and_then(|s| s.parse().ok())
						.unwrap_or(0);
			if !levels.contains(&level) {
				continue;
			}
			let admin_id = match relation.tags.get("ref:INSEE") {
				Some(val) => format!("admin:fr:{}",val),
				_ => format!("admin:osm:{}", relation.id)
			};
			let zip_code = match relation.tags.get("addr:postcode") {
				Some(val) => &val[..],
				_ => ""
			};

			let admin = mimirsbrunn::Admin {
							id: admin_id,
							level: level,
							name: relation.tags.get("name").unwrap_or(&"NC".to_string()).to_string(),
							zip_code: zip_code.to_string(),
							weight: 1
			};
			administrative_regions.push(admin);
		}
    }
    return administrative_regions;
}

fn main() {
    println!("importing adminstrative region into Mimir");
    let args: Args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());
	let map = args.flag_admin.iter().cloned().collect();
	let res = administartive_regions(&args.flag_input, &map);
    for ad in res{
    	println!("admins {:?}", ad.id);
    }
}
