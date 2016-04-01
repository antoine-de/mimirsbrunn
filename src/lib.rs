

extern crate rustc_serialize;
extern crate curl;

use rustc_serialize::json;

#[derive(RustcDecodable, RustcEncodable, Clone)]
pub struct Addr {
    pub id: String,
    pub house_number: String,
    pub name: String,
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
