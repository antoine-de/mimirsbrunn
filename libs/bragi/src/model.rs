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

use mimir;

use geojson;
use geo;
use std::rc::Rc;

#[derive(Serialize, Deserialize, Debug)]
pub struct Geocoding {
    version: String,
    query: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Feature {
    #[serde(rename="type")]
    pub feature_type: String,
    pub geometry: geojson::Geometry,
    pub properties: Properties,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Properties {
    pub geocoding: GeocodingResponse,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeocodingResponse {
    pub id: String,
    #[serde(rename="type")]
    pub place_type: String, // FIXME: use an enum?
    pub label: Option<String>,
    pub name: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub housenumber: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub street: Option<String>,
    pub postcode: Option<String>,
    pub city: Option<String>,
    pub citycode: Option<String>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub level: Option<u32>,
    // pub accuracy: Option<i32>,
    // pub district: Option<String>,
    // pub county: Option<String>,
    // pub state: Option<String>,
    // pub country: Option<String>,
    // pub geohash: Option<String>,
    pub administrative_regions: Vec<Rc<mimir::Admin>>,
    #[serde(skip_serializing_if="Vec::is_empty", default)]
    pub poi_types: Vec<mimir::PoiType>,
}

trait ToGeom {
    fn to_geom(&self) -> geojson::Geometry;
}

impl ToGeom for mimir::Place {
    fn to_geom(&self) -> geojson::Geometry {
        match self {
            &mimir::Place::Admin(ref admin) => admin.coord.to_geom(),
            &mimir::Place::Street(ref street) => street.coord.to_geom(),
            &mimir::Place::Addr(ref addr) => addr.coord.to_geom(),
            &mimir::Place::Poi(ref poi) => poi.coord.to_geom(),
            &mimir::Place::Stop(ref stop) => stop.coord.to_geom(),
        }
    }
}

impl ToGeom for geo::Coordinate<f64> {
    fn to_geom(&self) -> geojson::Geometry {
        geojson::Geometry::new(geojson::Value::Point(vec![self.y, self.x]))
    }
}

impl From<mimir::Place> for Feature {
    fn from(other: mimir::Place) -> Feature {
        let geom = other.to_geom();
        let geocoding = match other {
            mimir::Place::Admin(admin) => GeocodingResponse::from(admin),
            mimir::Place::Street(street) => GeocodingResponse::from(street),
            mimir::Place::Addr(addr) => GeocodingResponse::from(addr),
            mimir::Place::Poi(poi) => GeocodingResponse::from(poi),
            mimir::Place::Stop(poi) => GeocodingResponse::from(poi),
        };
        Feature {
            feature_type: "Feature".to_string(),
            geometry: geom,
            properties: Properties { geocoding: geocoding },
        }
    }
}

impl From<mimir::Admin> for GeocodingResponse {
    fn from(other: mimir::Admin) -> GeocodingResponse {
        let type_ = "city".to_string(); //to be improved: it can be something else
        let name = Some(other.name);
        let insee = Some(other.insee);
        let level = Some(other.level); //might be used for type_ and become useless
        let postcode = if other.zip_codes.is_empty() {
            None
        } else {
            Some(other.zip_codes.join(";"))
        };
        let label = Some(other.label);
        GeocodingResponse {
            id: other.id,
            citycode: insee,
            level: level,
            place_type: type_,
            name: name,
            postcode: postcode,
            label: label,
            housenumber: None,
            street: None,
            city: None,
            administrative_regions: vec![],
            poi_types: vec![],
        }
    }
}

fn get_city_name(admins: &Vec<Rc<mimir::Admin>>) -> Option<String> {
    // for the moment for the 'postcode' and the 'city', we take first admin
    // that has a postcode
    // TODO: change this (with a 'city tag in the admin ?')
    admins.iter()
        .find(|a| !a.zip_codes.is_empty())
        .or_else(|| admins.iter().next())
        .map(|admin| admin.name.clone())
}

fn get_citycode(admins: &Vec<Rc<mimir::Admin>>) -> Option<String> {
    admins.iter()
        .find(|a| !a.zip_codes.is_empty())
        .or_else(|| admins.iter().next())
        .map(|admin| admin.insee.clone())
}

impl From<mimir::Street> for GeocodingResponse {
    fn from(other: mimir::Street) -> GeocodingResponse {
        let type_ = "street".to_string();
        let name = Some(other.street_name);
        let label = Some(other.label);
        let admins = other.administrative_regions;
        let city = get_city_name(&admins);
        let postcode = if other.zip_codes.is_empty() {
            None
        } else {
            Some(other.zip_codes.join(";"))
        };
        let citycode = get_citycode(&admins);

        GeocodingResponse {
            id: other.id,
            citycode: citycode,
            level: None,
            place_type: type_,
            name: name,
            postcode: postcode,
            label: label,
            housenumber: None,
            street: None,
            city: city,
            administrative_regions: admins,
            poi_types: vec![],
        }
    }
}

impl From<mimir::Addr> for GeocodingResponse {
    fn from(other: mimir::Addr) -> GeocodingResponse {
        let type_ = "house".to_string();
        let label = Some(other.label);
        let housenumber = Some(other.house_number.to_string());
        let street_name = Some(other.street.street_name.to_string());
        let name = Some(format!("{} {}", other.house_number, other.street.street_name));
        let admins = other.street.administrative_regions;
        let city = get_city_name(&admins);
        let postcode = if other.zip_codes.is_empty() {
            None
        } else {
            Some(other.zip_codes.join(";"))
        };
        let citycode = get_citycode(&admins);

        GeocodingResponse {
            id: other.id,
            citycode: citycode,
            level: None,
            place_type: type_,
            name: name,
            postcode: postcode,
            label: label,
            housenumber: housenumber,
            street: street_name,
            city: city,
            administrative_regions: admins,
            poi_types: vec![],
        }
    }
}

impl From<mimir::Poi> for GeocodingResponse {
    fn from(other: mimir::Poi) -> GeocodingResponse {
        let type_ = "poi".to_string();
        let label = Some(other.label);
        let name = Some(other.name);
        let admins = other.administrative_regions;
        let city = get_city_name(&admins);
        let postcode = if other.zip_codes.is_empty() {
            None
        } else {
            Some(other.zip_codes.join(";"))
        };
        let citycode = get_citycode(&admins);

        GeocodingResponse {
            id: other.id,
            citycode: citycode,
            level: None,
            place_type: type_,
            name: name,
            postcode: postcode,
            label: label,
            housenumber: None,
            street: None,
            city: city,
            administrative_regions: admins,
            poi_types: vec![other.poi_type],
        }
    }
}

impl From<mimir::Stop> for GeocodingResponse {
    fn from(other: mimir::Stop) -> GeocodingResponse {
        let type_ = "public_transport:stop_area".to_string();
        let label = Some(other.label);
        let name = Some(other.name);
        let admins = other.administrative_regions;
        let city = get_city_name(&admins);
        let postcode = if other.zip_codes.is_empty() {
            None
        } else {
            Some(other.zip_codes.join(";"))
        };
        let citycode = get_citycode(&admins);

        GeocodingResponse {
            id: other.id,
            citycode: citycode,
            level: None,
            place_type: type_,
            name: name,
            postcode: postcode,
            label: label,
            housenumber: None,
            street: None,
            city: city,
            administrative_regions: admins,
            poi_types: vec![],
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Autocomplete {
    #[serde(rename="type")]
    format_type: String,
    geocoding: Geocoding,
    features: Vec<Feature>,
}

impl Autocomplete {
    pub fn new(q: String, features: Vec<Feature>) -> Autocomplete {
        // TODO couldn't we mode this function ? in Autocomplete ?
        Autocomplete {
            format_type: "FeatureCollection".to_string(),
            geocoding: Geocoding {
                version: "0.1.0".to_string(),
                query: Some(q),
            },
            features: features,
        }
    }
}

impl From<Vec<mimir::Place>> for Autocomplete {
    fn from(places: Vec<mimir::Place>) -> Autocomplete {
        Autocomplete::new("".to_string(),
                          places.into_iter().map(|p| Feature::from(p)).collect())
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Coord {
    pub lat: f64,
    pub lon: f64,
}

pub mod v1 {
    use mimir;
    use rs_es;

    // Note: I think this should be in api.rs but with the serde stuff it's easier for all
    // serde struct to be in the same file

    #[derive(Serialize, Deserialize, Debug)]
    pub struct EndPoint {
        pub description: String,
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct CustomError {
        pub short: String,
        pub long: String,
    }


    #[derive(Serialize, Deserialize, Debug)]
    pub enum V1Reponse {
        Error(CustomError),
        Response { description: String },
    }
    #[derive(Debug)]
    pub enum AutocompleteResponse {
        Error(CustomError),
        Autocomplete(super::Autocomplete),
    }


    use serde;
    impl serde::Serialize for AutocompleteResponse {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
            where S: serde::Serializer
        {
            match self {
                &AutocompleteResponse::Autocomplete(ref a) => serializer.serialize_some(a),
                &AutocompleteResponse::Error(ref e) => serializer.serialize_some(e),
            }
        }
    }

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Status {
        pub version: String,
        pub es: String,
        pub status: String,
    }

    impl From<Result<Vec<mimir::Place>, rs_es::error::EsError>> for AutocompleteResponse {
        fn from(r: Result<Vec<mimir::Place>, rs_es::error::EsError>) -> AutocompleteResponse {
            match r {
                Ok(places) => AutocompleteResponse::Autocomplete(super::Autocomplete::from(places)),
                Err(e) => {
                    AutocompleteResponse::Error(CustomError {
                        short: "query error".to_string(),
                        long: format!("invalid query {:?}", e),
                    })
                }
            }
        }
    }
}
