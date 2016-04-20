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
use jsonway;
use rustc_serialize::json::ToJson;
use rustless::server::status;
use std::error::Error;
use rustless::{
    Api, Nesting, Versioning
};

pub fn root() -> rustless::Api {
    Api::build(|api| {
        api.get("", |endpoint| {
            endpoint.handle(|client, params| {
                    println!("endpoint");
                    client.text("/".to_string())
            })
        });

        api.error_formatter(|error, _media| {
            println!("rooooh on a une erreur sur le endpoint principal: {:?}", error);

            Some(rustless::Response::from(status::StatusCode::BadRequest, Box::new("elle est pas cool ton erreur".to_string())))
        });
        api.mount(v1());
    })
}

#[derive(Serialize, Deserialize, Debug)]
struct CustomError {
    short: String,
    long: String
}

#[derive(Serialize, Deserialize, Debug)]
struct Autocomplete {
    todo_change_name_type: String,
    version: String,
    query: String,
    // features: Array(sources)
}

#[derive(Serialize, Deserialize, Debug)]
enum V1Reponse {
    Error(CustomError),
    Response {
        description: String
    }
}

#[derive(Serialize, Deserialize, Debug)]
enum AutocompleteResponse {
    Error(CustomError)
}

#[derive(Serialize, Deserialize, Debug)]
struct Status {
    version: String,
    status: String
}

pub fn v1() -> rustless::Api {
    Api::build(|api| {
        api.version("v1", Versioning::Path);

        // Add error formatter to send validation errors back to the client
        api.error_formatter(|error, _media| {
            println!("rooooh on a une erreur: {:?}", error);

            match error.downcast::<rustless::errors::Validation>() {
                Some(val_err) => {
                    return Some(rustless::Response::from_json(status::StatusCode::BadRequest, &jsonway::object(|json| {
                        json.set_json("errors", val_err.reason.to_json())
                    }).unwrap()))
                },
                None => ()
            }
            println!("c'est une error not match");
            Some(rustless::Response::from(status::StatusCode::Unauthorized, Box::new("elle est pas cool ton erreur".to_string())))
        /*Some(rustless::Response::from_json(status::StatusCode::Ok, &jsonway::object(|json| {
                json.set_json("errors", error.description().to_json())
            }).unwrap()))*/
        });

        api.get("", |endpoint| {
            endpoint.handle(|client, params| {
                    println!("heho v1");
                    client.text("v1/".to_string())
            })
        });
        api.before(|_, _| {

            println!("on a recu une requete");
            Ok(())
        });
        api.mount(Api::build(|toto_api| {
            toto_api.get("toto/:toto", |endpoint| {
                endpoint.handle(|client, params| {

                        println!("v1/toto");
                        client.text("youhouuuuuuuuuuuu toto".to_string())
                })
            });

            toto_api.after(|client, _params| {
                println!("after toto");
                Ok(())
            });
        }));
    })
}
