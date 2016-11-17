// Copyright © 2016, Canal TP and/or its affiliates. All rights reserved.
//
// This file is part of Navitia,
//     the software to build cool stuff with public transport.
//
// Hope you'll enjoy and contribute to this project,
//     powered by Canal TP (www.canaltp.fr).
// Help us simplify mobility and open public transport:
//     a non ending quest to the responsive locomotion way of traveling!
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
//
// Stay tuned using
// twitter @navitia
// IRC #navitia on freenode
// https://groups.google.com/d/forum/navitia
// www.navitia.io

use geo;
use serde;
use std::rc::Rc;
use std::cell::Cell;
use serde::ser::Serializer;
use std::cmp::Ordering;

// Note: this file is needed to use serde in rust stable
// cf mimirsbrunn/build.rs for explanations

pub trait Incr: Clone {
    fn id(&self) -> &str;
    fn incr(&mut self);
}

/// Object stored in elastic search
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Place {
    Admin(Admin),
    Street(Street),
    Addr(Addr),
    Poi(Poi),
}

impl Place{
    pub fn is_admin(&self) -> bool {
        match *self {
            Place::Admin(_) => true,
            _ => false
        }
    }
    pub fn is_street(&self) -> bool {
        match *self {
            Place::Street(_) => true,
            _ => false
        }
    }
    pub fn is_addr(&self) -> bool {
        match *self {
            Place::Addr(_) => true,
            _ => false
        }
    }
    pub fn is_poi(&self) -> bool {
        match *self {
            Place::Poi(_) => true,
            _ => false
        }
    }
    pub fn label(&self) -> &str {
        match *self {
            Place::Admin(ref o) => o.label(),
            Place::Street(ref o) => o.label(),
            Place::Addr(ref o) => o.label(),
            Place::Poi(ref o) => o.label(),
        }
    }

    pub fn admins(&self) -> Vec<Rc<Admin>> {
        match *self {
            Place::Admin(ref o) => o.admins(),
            Place::Street(ref o) => o.admins(),
            Place::Addr(ref o) => o.admins(),
            Place::Poi(ref o) => o.admins(),
        }
    }
}

pub trait DocType {
    fn doc_type() -> &'static str; // provides the elasticsearch type name
}

pub trait EsId {
    fn es_id(&self) -> Option<String>; // provides the elasticsearch id
}

pub trait Members{
    fn label(&self) -> &str;
    fn admins(&self) -> Vec<Rc<Admin>>;
}

impl<'a, T: DocType> DocType for &'a T {
    fn doc_type() -> &'static str {
        T::doc_type()
    }
}
impl<'a, T: EsId> EsId for &'a T {
    fn es_id(&self) -> Option<String> {
        T::es_id(self)
    }
}
impl<'a, T: DocType> DocType for Rc<T> {
    fn doc_type() -> &'static str {
        T::doc_type()
    }
}
impl<'a, T: EsId> EsId for Rc<T> {
    fn es_id(&self) -> Option<String> {
        T::es_id(self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Poi {
    pub id: String,
    pub label: String,
    pub name: String,
    pub coord: Coord,
    pub administrative_regions: Vec<Rc<Admin>>,
    pub weight: u32,
    pub zip_codes: Vec<String>,
}

impl DocType for Poi {
    fn doc_type() -> &'static str {
        "poi"
    }
}

impl EsId for Poi {
    fn es_id(&self) -> Option<String> {
        Some(self.id.clone())
    }
}

impl Members for Poi {
    fn label(&self) -> &str {
        &self.label
    }
    fn admins(&self) -> Vec<Rc<Admin>> {
        self.administrative_regions.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Admin {
    pub id: String,
    pub insee: String,
    pub level: u32,
    pub label: String,
    pub zip_codes: Vec<String>,
    #[serde(serialize_with="custom_cell_serialize", skip_deserializing)]
    //Attribut weight is used in elastic search to sort the result. It is absent in the response
    //of navitia (jormungandr) and hence deserializing is not necessary
    pub weight: Cell<u32>,
    pub coord: Coord,
    #[serde(skip_serializing, skip_deserializing)]
    pub boundary: Option<geo::MultiPolygon>,
}

impl Ord for Admin {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl PartialOrd for Admin {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for Admin {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Members for Admin {
    fn label(&self) -> &str {
        &self.label
    }
    fn admins(&self) -> Vec<Rc<Admin>> {
        vec![Rc::new(self.clone())]
    }
}

impl Eq for Admin {}

fn custom_cell_serialize<S>(cell: &Cell<u32>, serializer: &mut S) -> Result<(), S::Error> where S: Serializer {
	// we can serialize the cell as a u32, since the reference is important only while loading the data
	// in ES but not in bragi
	serializer.serialize_u32(cell.get())
}

impl EsId for Admin {
    fn es_id(&self) -> Option<String> {
        Some(self.id.clone())
    }
}

impl DocType for Admin {
    fn doc_type() -> &'static str {
        "admin"
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Street {
    pub id: String,
    pub street_name: String,
    pub administrative_regions: Vec<Rc<Admin>>,
    pub label: String,
    pub weight: u32,
    pub coord: Coord,
    pub zip_codes: Vec<String>,
}
impl Incr for Street {
    fn id(&self) -> &str {
        &self.id
    }
    fn incr(&mut self) {
        self.weight += 1;
    }
}
impl DocType for Street {
    fn doc_type() -> &'static str {
        "street"
    }
}

impl EsId for Street {
    fn es_id(&self) -> Option<String> {
        None
    }
}

impl Members for Street {
    fn label(&self) -> &str {
        &self.label
    }
    fn admins(&self) -> Vec<Rc<Admin>> {
        self.administrative_regions.clone()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Addr {
    pub id: String,
    pub house_number: String,
    pub street: Street,
    pub label: String,
    pub coord: Coord,
    pub weight: u32,
    pub zip_codes: Vec<String>,
}

impl DocType for Addr {
    fn doc_type() -> &'static str {
        "addr"
    }
}

impl EsId for Addr {
    fn es_id(&self) -> Option<String> {
        None
    }
}

impl Members for Addr {
    fn label(&self) -> &str {
        &self.label
    }
    fn admins(&self) -> Vec<Rc<Admin>> {
        self.street.admins()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliasOperations {
    pub actions: Vec<AliasOperation>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliasOperation {
    #[serde(skip_serializing_if="Option::is_none")]
    pub add: Option<AliasParameter>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub remove: Option<AliasParameter>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AliasParameter{
    pub index: String,
    pub alias: String,

}

// we want a custom serialization for coords, and so far the cleanest way
// to do this that has been found is to wrap the coord in another struct
#[derive(Debug, Clone)]
pub struct Coord(pub geo::Coordinate);
impl Coord {
    pub fn new(lat: f64, lon: f64) -> Coord {
        Coord(geo::Coordinate {x: lat, y: lon})
    }
    pub fn lat(&self) -> f64 {
        self.x
    }
    pub fn lon(&self) -> f64 {
        self.y
    }
}

impl ::std::ops::Deref for Coord {
    type Target = geo::Coordinate;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl serde::Serialize for Coord {
  fn serialize<S>(&self, serializer: &mut S) -> Result<(), S::Error>
        where S: serde::Serializer {
      let mut state = try!(serializer.serialize_struct("Coord", 2));
      try!(serializer.serialize_struct_elt(&mut state, "lat", &self.0.x));
      try!(serializer.serialize_struct_elt(&mut state, "lon", &self.0.y));
      serializer.serialize_struct_end(state)
  }
}

enum GeoCoordField {
    X,
    Y,
}

impl serde::Deserialize for GeoCoordField {
    fn deserialize<D>(deserializer: &mut D) -> Result<GeoCoordField, D::Error>
        where D: serde::de::Deserializer
    {
        struct GeoCoordFieldVisitor;

        impl serde::de::Visitor for GeoCoordFieldVisitor {
            type Value = GeoCoordField;

            fn visit_str<E>(&mut self, value: &str) -> Result<GeoCoordField, E>
                where E: serde::de::Error
            {
                match value {
                    "lat" => Ok(GeoCoordField::X),
                    "lon" => Ok(GeoCoordField::Y),
                    _ => Err(serde::de::Error::custom("expected lon or lat")),
                }
            }
        }

        deserializer.deserialize(GeoCoordFieldVisitor)
    }
}

impl serde::Deserialize for Coord {
  fn deserialize<D>(deserializer: &mut D) -> Result<Coord, D::Error>
      where D: serde::de::Deserializer
  {
      static FIELDS: &'static [&'static str] = &["lat", "lon"];
      deserializer.deserialize_struct("Coord", FIELDS, GeoCoordDeserializerVisitor)
  }
}
struct GeoCoordDeserializerVisitor;

impl serde::de::Visitor for GeoCoordDeserializerVisitor {
    type Value = Coord;

    fn visit_map<V>(&mut self, mut visitor: V) -> Result<Coord, V::Error>
        where V: serde::de::MapVisitor
    {
        let mut x = None;
        let mut y = None;

        loop {
            match try!(visitor.visit_key()) {
                Some(GeoCoordField::X) => { x = Some(try!(visitor.visit_value())); }
                Some(GeoCoordField::Y) => { y = Some(try!(visitor.visit_value())); }
                None => { break; }
            }
        }

        let x = match x {
            Some(x) => x,
            None => try!(visitor.missing_field("x")),
        };

        let y = match y {
            Some(y) => y,
            None => try!(visitor.missing_field("y")),
        };

        try!(visitor.end());

        Ok(Coord(geo::Coordinate{ x: x, y: y }))
    }
}
