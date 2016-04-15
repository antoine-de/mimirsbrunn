// Copyright © 2014, Canal TP and/or its affiliates. All rights reserved.
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

use curl;
use rustc_serialize::Encodable;

trait Incr: Clone {
    fn id(&self) -> &str;
    fn incr(&mut self);
}

#[derive(RustcDecodable, RustcEncodable)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
}

pub type CurlResult = Result<curl::http::Response, curl::ErrCode>;
