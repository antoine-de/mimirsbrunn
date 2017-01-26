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
use rustless;
use serde;
use serde_json;
use rustless::server::{status, header};
use rustless::{Api, Nesting};
use valico::json_dsl;
use valico::common::error as valico_error;
use super::query;
use model::v1::*;
use model;

const MAX_LAT: f64 = 180f64;
const MIN_LAT: f64 = -180f64;

const MAX_LON: f64 = 90f64;
const MIN_LON: f64 = -90f64;
const DEFAULT_LIMIT: u64 = 10u64;
const DEFAULT_OFFSET: u64 = 0u64;

fn render<T>(mut client: rustless::Client,
             obj: T)
             -> Result<rustless::Client, rustless::ErrorResponse>
    where T: serde::Serialize
{
    client.set_json_content_type();
    client.set_header(header::AccessControlAllowOrigin::Any);
    client.text(serde_json::to_string(&obj).unwrap())
}

pub struct ApiEndPoint {
    pub es_cnx_string: String,
}

impl ApiEndPoint {
    pub fn root(&self) -> rustless::Api {
        Api::build(|api| {
            api.get("", |endpoint| {
                endpoint.handle(|client, _params| {
                    let desc = EndPoint { description: "autocomplete service".to_string() };
                    render(client, desc)
                })
            });

            api.error_formatter(|error, _media| {
                let err = if error.is::<rustless::errors::Validation>() {
                    let val_err = error.downcast::<rustless::errors::Validation>().unwrap();
                    // TODO better message, we shouldn't use {:?} but access the `path`
                    // and `detail` of all errrors in val_err.reason
                    CustomError {
                        short: "validation error".to_string(),
                        long: format!("invalid arguments {:?}", val_err.reason),
                    }
                } else {
                    CustomError {
                        short: "bad_request".to_string(),
                        long: format!("bad request, error: {}", error),
                    }
                };
                let mut resp = rustless::Response::from(status::StatusCode::BadRequest,
                                                        Box::new(serde_json::to_string(&err)
                                                            .unwrap()));
                resp.set_json_content_type();
                Some(resp)
            });
            api.mount(self.v1());
        })
    }

    fn v1(&self) -> rustless::Api {
        Api::build(|api| {
            api.mount(self.status());
            api.mount(self.autocomplete());
        })
    }

    fn status(&self) -> rustless::Api {
        Api::build(|api| {
            api.get("status", |endpoint| {
                let cnx = self.es_cnx_string.clone();
                endpoint.handle(move |client, _params| {
                    let status = Status {
                        version: env!("CARGO_PKG_VERSION").to_string(),
                        es: cnx.to_string(),
                        status: "good".to_string(),
                    };
                    render(client, status)
                })
            });
        })
    }

    fn autocomplete(&self) -> rustless::Api {
        Api::build(|api| {
            api.post("autocomplete", |endpoint| {
                endpoint.params(|params| {
                    params.opt_typed("q", json_dsl::string());
                    params.opt_typed("limit", json_dsl::u64());
                    params.opt_typed("offset", json_dsl::u64());
                    params.req("geometry", |geometry| {
                        geometry.coerce(json_dsl::object());
                        geometry.nest(|params| {
                            params.req("type", |geojson_type| {
                                geojson_type.coerce(json_dsl::string());
                                geojson_type.allow_values(&["Polygon".to_string()]);
                            });
                        });
                        geometry.nest(|params| {
                            params.req("coordinates", |shape| {
                                shape.coerce(json_dsl::array());
                                shape.validate_with(|val, path| {
                                    check_coordinates(val, path, "Coordinates is invalid")
                                });
                            });
                        });
                    })
                });

                let cnx = self.es_cnx_string.clone();
                endpoint.handle(move |client, params| {
                    let q = params.find("q").and_then(|val| val.as_str()).unwrap_or("").to_string();
                    let offset = params.find("offset")
                        .and_then(|val| val.as_u64())
                        .unwrap_or(DEFAULT_OFFSET);
                    let limit =
                        params.find("limit").and_then(|val| val.as_u64()).unwrap_or(DEFAULT_LIMIT);
                    let geometry = params.find_path(&["geometry"]).unwrap();
                    let coordinates =
                        geometry.find_path(&["coordinates"]).unwrap().as_array().unwrap();
                    let mut shape = Vec::new();
                    for ar in coordinates[0].as_array().unwrap() {
                        // (Lat, Lon)
                        shape.push((ar.as_array().unwrap()[1].as_f64().unwrap(),
                                    ar.as_array().unwrap()[0].as_f64().unwrap()));
                    }
                    let model_autocomplete =
                        query::autocomplete(&q, offset, limit, None, &cnx, Some(shape));

                    let response = model::v1::AutocompleteResponse::from(model_autocomplete);
                    render(client, response)
                })
            });
            api.get("autocomplete", |endpoint| {
                endpoint.params(|params| {
                    params.req_typed("q", json_dsl::string());
                    params.opt_typed("limit", json_dsl::u64());
                    params.opt_typed("offset", json_dsl::u64());
                    params.opt("lon", |lon| {
                        lon.coerce(json_dsl::f64());
                        lon.validate_with(|val, path| {
                            check_bound(val, path, MIN_LON, MAX_LON, "lon is not a valid longitude")
                        });
                    });

                    params.opt("lat", |lat| {
                        lat.coerce(json_dsl::f64());
                        lat.validate_with(|val, path| {
                            check_bound(val, path, MIN_LAT, MAX_LAT, "lat is not a valid latitude")
                        });
                    });
                    params.validate_with(|val, path| {
                        // if we have a lat we should have a lon (and the opposite)
                        if let Some(obj) = val.as_object() {
                            let has_lon = obj.get("lon").is_some();
                            let has_lat = obj.get("lat").is_some();
                            if has_lon ^ has_lat {
                                Err(vec![Box::new(json_dsl::errors::WrongValue {
                                             path: path.to_string(),
                                             detail: Some("you need to provide a lon AND a lat \
                                                           if you provide one of them"
                                                 .to_string()),
                                         })])
                            } else {
                                Ok(())
                            }
                        } else {
                            unreachable!("should never happen, already checked");
                        }
                    });
                });
                let cnx = self.es_cnx_string.clone();
                endpoint.handle(move |client, params| {
                    let q = params.find("q").and_then(|val| val.as_str()).unwrap_or("").to_string();
                    let offset = params.find("offset")
                        .and_then(|val| val.as_u64())
                        .unwrap_or(DEFAULT_OFFSET);
                    let limit =
                        params.find("limit").and_then(|val| val.as_u64()).unwrap_or(DEFAULT_LIMIT);
                    let lon = params.find("lon").and_then(|p| p.as_f64());
                    let lat = params.find("lat").and_then(|p| p.as_f64());
                    // we have already checked that if there is a lon, lat
                    // is not None so we can unwrap
                    let coord = lon.and_then(|lon| {
                        Some(model::Coord {
                            lon: lon,
                            lat: lat.unwrap(),
                        })
                    });
                    let model_autocomplete =
                        query::autocomplete(&q, offset, limit, coord, &cnx, None);

                    let response = model::v1::AutocompleteResponse::from(model_autocomplete);
                    render(client, response)
                })
            });
        })
    }
}

fn check_bound(val: &serde_json::Value,
               path: &str,
               min: f64,
               max: f64,
               error_msg: &str)
               -> Result<(), valico_error::ValicoErrors> {
    if let Some(lon) = val.as_f64() {
        if min <= lon && lon <= max {
            Ok(())
        } else {
            Err(vec![Box::new(json_dsl::errors::WrongValue {
                         path: path.to_string(),
                         detail: Some(error_msg.to_string()),
                     })])
        }
    } else {
        unreachable!("should never happen, already checked");
    }
}

fn check_coordinates(val: &serde_json::Value,
                     path: &str,
                     error_msg: &str)
                     -> Result<(), valico_error::ValicoErrors> {

    if !val.is_array() {
        return Err(vec![Box::new(json_dsl::errors::WrongType {
                            path: path.to_string(),
                            detail: error_msg.to_string(),
                        })]);
    }
    let array = val.as_array().unwrap();
    if array.is_empty() {
        return Err(vec![Box::new(json_dsl::errors::WrongValue {
                            path: path.to_string(),
                            detail: Some(error_msg.to_string()),
                        })]);
    }

    for arr0 in array {
        if !arr0.is_array() {
            return Err(vec![Box::new(json_dsl::errors::WrongType {
                                path: path.to_string(),
                                detail: error_msg.to_string(),
                            })]);
        }
        let arr1 = arr0.as_array().unwrap();
        if arr1.is_empty() {
            return Err(vec![Box::new(json_dsl::errors::WrongValue {
                                path: path.to_string(),
                                detail: Some(error_msg.to_string()),
                            })]);
        }
        for arr2 in arr1 {
            if !arr2.is_array() {
                return Err(vec![Box::new(json_dsl::errors::WrongType {
                                    path: path.to_string(),
                                    detail: error_msg.to_string(),
                                })]);
            }
            let lonlat = arr2.as_array().unwrap();
            if lonlat.len() != 2 {
                return Err(vec![Box::new(json_dsl::errors::WrongValue {
                                    path: path.to_string(),
                                    detail: Some(error_msg.to_string()),
                                })]);
            }

            if !(lonlat[0].is_f64() && lonlat[1].is_f64()) {
                return Err(vec![Box::new(json_dsl::errors::WrongType {
                                    path: path.to_string(),
                                    detail: error_msg.to_string(),
                                })]);
            }
            let lon = lonlat[0].as_f64().unwrap();
            let lat = lonlat[1].as_f64().unwrap();
            if !(MIN_LON <= lon && lon <= MAX_LON) {
                return Err(vec![Box::new(json_dsl::errors::WrongValue {
                                    path: path.to_string(),
                                    detail: Some(error_msg.to_string()),
                                })]);
            }
            if !(MIN_LAT <= lat && lat <= MAX_LAT) {
                return Err(vec![Box::new(json_dsl::errors::WrongValue {
                                    path: path.to_string(),
                                    detail: Some(error_msg.to_string()),
                                })]);
            }
        }
    }
    Ok(())
}
