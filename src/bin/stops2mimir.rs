extern crate rustc_serialize;
extern crate docopt;
extern crate csv;
extern crate mimir;
use mimir::rubber::Rubber;
#[macro_use]
extern crate log;

use docopt::Docopt;

const USAGE: &'static str =
    "
Usage:
    stops2mimir --help
    stops2mimir --input=<file> \
     [--connection-string=<connection-string>] [--dataset=<dataset>]

Options:
    -h, --help              \
     Show this message.
    -i, --input=<file>      NTFS stops.txt file.
    -c, \
     --connection-string=<connection-string>
                            Elasticsearch \
     parameters, [default: http://localhost:9200/munin]
    -d, --dataset=<dataset>
";

#[derive(Debug, RustcDecodable)]
struct Args {
    flag_input: String,
    flag_dataset: String,
    flag_connection_string: String,
}

struct StopPointIter<'a, R: std::io::Read + 'a> {
    iter: csv::StringRecords<'a, R>,
    stop_id_pos: usize,
    stop_lat_pos: usize,
    stop_lon_pos: usize,
    stop_name_pos: usize,
    location_type_pos: Option<usize>,
    stop_visible_pos: Option<usize>,
}
impl<'a, R: std::io::Read + 'a> StopPointIter<'a, R> {
    fn new(r: &'a mut csv::Reader<R>) -> Option<Self> {
        let headers = if let Ok(hs) = r.headers() {
            hs
        } else {
            return None;
        };
        let get = |name| headers.iter().position(|s| s == name);
        let stop_id_pos = if let Some(pos) = get("stop_id") {
            pos
        } else {
            return None;
        };
        let stop_lat_pos = if let Some(pos) = get("stop_lat") {
            pos
        } else {
            return None;
        };
        let stop_lon_pos = if let Some(pos) = get("stop_lon") {
            pos
        } else {
            return None;
        };
        let stop_name_pos = if let Some(pos) = get("stop_name") {
            pos
        } else {
            return None;
        };

        Some(StopPointIter {
            iter: r.records(),
            stop_id_pos: stop_id_pos,
            stop_lat_pos: stop_lat_pos,
            stop_lon_pos: stop_lon_pos,
            stop_name_pos: stop_name_pos,
            location_type_pos: get("location_type"),
            stop_visible_pos: get("visible"),
        })
    }
    fn get_location_type(&self, record: &[String]) -> Option<u8> {
        self.location_type_pos.and_then(|pos| record.get(pos).and_then(|s| s.parse().ok()))
    }
    fn get_visible(&self, record: &[String]) -> Option<u8> {
        self.stop_visible_pos.and_then(|pos| record.get(pos).and_then(|s| s.parse().ok()))
    }
}
impl<'a, R: std::io::Read + 'a> Iterator for StopPointIter<'a, R> {
    type Item = csv::Result<mimir::Stop>;
    fn next(&mut self) -> Option<Self::Item> {
        fn get(record: &[String], pos: usize) -> csv::Result<&str> {
            match record.get(pos) {
                Some(s) => Ok(&s),
                None => Err(csv::Error::Decode(format!("Failed accessing record '{}'.", pos))),
            }
        }
        fn parse_f64(s: &str) -> csv::Result<f64> {
            s.parse()
                .map_err(|_| csv::Error::Decode(format!("Failed converting '{}' from str.", s)))
        }
        fn is_stop_area(location_type: &Option<u8>, visible: &Option<u8>) -> csv::Result<bool> {
            if (*location_type == Some(1)) && (*visible == Some(0)) {
                Ok(true)
            } else {
                Err(csv::Error::Decode("Not stop_area.".to_string()))
            }
        }
        self.iter.next().map(|r| {
            r.and_then(|r| {
                let stop_id = try!(get(&r, self.stop_id_pos));
                let stop_lat = try!(get(&r, self.stop_lat_pos));
                let stop_lat = try!(parse_f64(stop_lat));
                let stop_lon = try!(get(&r, self.stop_lon_pos));
                let stop_lon = try!(parse_f64(stop_lon));
                let stop_name = try!(get(&r, self.stop_name_pos));
                let location_type = self.get_location_type(&r);
                let visible = self.get_visible(&r);
                try!(is_stop_area(&location_type, &visible));
                Ok(mimir::Stop {
                    id: stop_id.to_string(),
                    coord: mimir::Coord::new(stop_lat, stop_lon),
                    label: stop_name.to_string(),
                    weight: 1,
                    zip_codes: vec![],
                    administrative_regions: vec![],
                    name: stop_name.to_string(),
                })
            })
        })
    }
}

fn main() {
    println!("Launching stops2mimir...");

    let args: Args = Docopt::new(USAGE)
        .and_then(|dopt| dopt.decode())
        .unwrap_or_else(|e| e.exit());
	println!("args: {:?}", args);
    println!("creation of indexes");
    let mut rubber = Rubber::new(&args.flag_connection_string);

    let mut rdr = csv::Reader::from_file(args.flag_input)
        .unwrap()
        .double_quote(true);

    let stops: Vec<mimir::Stop> = StopPointIter::new(&mut rdr)
        .expect("Can't find needed fields in the header.")
        .filter_map(|rc| {
            rc.map_err(|e| println!("error at csv line decoding : {}", e))
                .ok()
        })
        .collect();

    println!("Importing stops into Mimir");
    let nb_stops = rubber.index("stops", &args.flag_dataset, stops.iter())
        .unwrap();

    println!("Nb of indexed stops: {}", nb_stops);

}
