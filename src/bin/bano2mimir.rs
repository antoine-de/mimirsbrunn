// Copyright © 2016, Canal TP and/or its affiliates. All rights reserved.
//
// LICENCE: This program is free software; you can redistribute it
// and/or modify it under the terms of the GNU Affero General Public
// License as published by the Free Software Foundation, either
// version 3 of the License, or (at your option) any later version.
//
// This program is distributed in the hope that it will be useful, but
// WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU
// Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public
// License along with this program. If not, see
// <http://www.gnu.org/licenses/>.

extern crate docopt;
extern crate csv;
extern crate rustc_serialize;
extern crate curl;
extern crate mimirsbrunn;

use std::path::Path;

pub fn index<I: Iterator<Item = mimirsbrunn::Addr>>(iter: I) -> Result<u32, curl::ErrCode> {
    unreachable!()
}

fn index_bano(files: &[String]) {
    println!("purge and create Munin...");
    mimirsbrunn::purge_and_create_munin().unwrap();
    println!("Munin purged and created.");

    for f in files.iter() {
        println!("importing {}...", f);
        let mut rdr = csv::Reader::from_file(&Path::new(&f)).unwrap().has_headers(false);


    }
}

#[derive(RustcDecodable, Debug)]
struct Args {
    arg_bano_files: Vec<String>
}

static USAGE: &'static str = "
Usage:
    bano2mimir <bano-files>...
";

fn main() {
    println!("c'est tipar");

    let args: Args = docopt::Docopt::new(USAGE)
        .and_then(|d| d.decode())
        .unwrap_or_else(|e| e.exit());

    index_bano(&args.arg_bano_files);
}
