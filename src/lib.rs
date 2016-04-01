

extern crate rustc_serialize;
extern crate curl;

use rustc_serialize::json;
use rustc_serialize::Encodable;
use std::collections::HashMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};

trait Incr: Clone {
    fn id(&self) -> &str;
    fn incr(&mut self);
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct Coord { pub lat: f64, pub lon: f64 }

#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Admin {
    pub id: String,
    pub level: u32,
    pub name: String,
    pub zip_code: String,
    pub weight: u32,
}
impl Incr for Admin {
    fn id(&self) -> &str { &self.id }
    fn incr(&mut self) { self.weight += 1; }
}

#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Street {
    pub id: String,
    pub street_name: String,
    pub name: String,
    pub administrative_region: Admin,
    pub weight: u32,
}
impl Incr for Street {
    fn id(&self) -> &str { &self.id }
    fn incr(&mut self) { self.weight += 1; }
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct Addr {
    pub id: String,
    pub house_number: String,
    pub street: Street,
    pub name: String,
    pub coord: Coord,
    pub weight: u32,
}

pub fn purge_and_create_munin() -> Result<(), curl::ErrCode> {
    // first, we must delete with its own handle the old munin
    try!(curl::http::handle().delete("http://localhost:9200/munin").exec());

    let analysis = include_str!("../json/settings.json");
    assert!(analysis.parse::<json::Json>().is_ok());
    let res = try!(curl::http::handle().put("http://localhost:9200/munin", analysis).exec());
    assert!(res.get_code() == 200, "Error adding analysis: {}", res);

    Ok(())
}

fn push_bulk<'a, T: Encodable>(s: &mut String, elt: &T) {
    s.push_str("{index: {}}\n");
    s.push_str(&json::encode(elt).unwrap());
    s.push('\n');
}
fn bulk_index<'a, T, I>(url: &str, mut iter: I) -> Result<u32, curl::ErrCode>
    where T: Encodable, I: Iterator<Item = T>
{
    let url = format!("{}/_bulk", url);
    let mut handle = curl::http::handle();
    let mut nb = 0;
    let mut chunk = String::new();
    loop {
        chunk.clear();
        let addr = match iter.next() { Some(a) => a, None => break };
        push_bulk(&mut chunk, &addr);
        nb += 1;
        for addr in iter.by_ref().take(1000) {
            push_bulk(&mut chunk, &addr);
            nb += 1;
        }
        let res = try!(handle.post(&*url, &chunk).exec());
        assert!(res.get_code() != 201, format!("result of bulk insert is not 201: {}", res));
    }
    Ok(nb)
}

fn upsert<T: Incr>(elt: &T, map: &mut HashMap<String, T>) {
    match map.entry(elt.id().to_string()) {
        Vacant(e) => { e.insert(elt.clone()); }
        Occupied(mut e) => e.get_mut().incr()
    }
}

pub fn index<I: Iterator<Item = Addr>>(iter: I) -> Result<u32, curl::ErrCode> {
    let mut admins = HashMap::new();
    let mut streets = HashMap::new();
    try!(bulk_index("http://localhost:9200/munin/addr", iter.inspect(|addr| {
        upsert(&addr.street.administrative_region, &mut admins);
        upsert(&addr.street, &mut streets);
    })));
    try!(bulk_index("http://localhost:9200/munin/admin", admins.into_iter().map(|e| e.1)));
    bulk_index("http://localhost:9200/munin/street", streets.into_iter().map(|e| e.1))
}
