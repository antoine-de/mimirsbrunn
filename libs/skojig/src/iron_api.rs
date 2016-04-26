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

use iron;
use iron::{Chain, Request, Response, IronResult, AfterMiddleware, IronError, status, typemap};
use router::{Router, NoRoute};
use model::v1;
use iron::Set;
use serde_json;
use serde;
use query;
use urlencoded::UrlEncodedQuery;
use iron::prelude::Plugin;

struct Custom404;

fn parse_autocomplete_params(req: &mut Request) -> Result<AutocompleteParams, String> {
    // TODO it would be nice to have a automatic parse method
    let hasmap = try!(req.get_ref::<UrlEncodedQuery>().map_err(|e| "unable to parse arguments".to_string()));

    Ok(AutocompleteParams {
        q: try!(hasmap.get("q").ok_or("missing mandatory parameter, q")).first().unwrap().clone()
    })
}

fn autocomplete(req: &mut Request) -> IronResult<Response> {
    //let params = itry!(parse_autocomplete_params(req), status::BadRequest); // TODO propagate error

    let autocomplete = query::autocomplete("bob".to_string(), None);
    Ok(json_response(status::Ok, autocomplete))
}
// inspired by https://github.com/sunng87/iron-json-response but more strongly typed and simplified
/*struct TypedResponseMiddleware<T>;
impl<T> typemap::Key for TypedResponseMiddleware<T> {
    type Value = T;
}*/

pub fn root() -> Chain {
    let mut chain = Chain::new(api());

    chain.link_after(Custom404);
    //chain.link_after(TypedResponseMiddleware::<v1::CustomError>);

    chain
}
/*
struct Json<T> {
    s: status::Status,
    T obj;
}

impl iron::modifier::Modifier<Response> for Json {
    pub fn modify(self, r: &mut Response) {
        r.body = Some(Box::new(serde_json::to_string(&self.obj).unwrap()));
        r.status = s;
        r.headers.set(iron::headers::ContentType("application/json; charset=utf-8".parse().unwrap()));
    }
}*/

fn json_response<T>(s: status::Status, obj: T) -> Response
    where T: serde::Serialize
{
    let mut r = Response::with((s, serde_json::to_string(&obj).unwrap()));
    r.headers.set(iron::headers::ContentType("application/json; charset=utf-8".parse().unwrap()));
    r
}

struct AutocompleteParams {
    q: String,
}

struct FirstMiddleware;
impl iron::BeforeMiddleware for FirstMiddleware {

    fn before(&self, req: &mut Request) -> IronResult<()> {

        info!("prems'");
        Ok(())
    }
}

fn api() -> Chain {
    let mut router = Router::new();
    router.get("/", |_: &mut Request| {
        Ok(json_response(status::Ok, v1::EndPoint {
            description: "autocomplete service".to_string()
        }))
    });
    router.get("/v1", |_: &mut Request| {
        Ok(json_response(status::Ok, v1::V1Reponse::Response {
             description: "api version 1".to_string()
         }))
    });
    router.get("/v1/status", |_: &mut Request| {
        Ok(json_response(status::Ok, v1::Status {
            version: "14".to_string(),
            status: "good".to_string(),
        }))
    });
    let mut autocomplete_chain = Chain::new(autocomplete);
    router.get("/v1/autocomplete", autocomplete_chain);

    let mut main_chain = Chain::new(router);
    main_chain.link_before(FirstMiddleware);

    main_chain
}

impl AfterMiddleware for Custom404 {
    fn catch(&self, req: &mut Request, err: IronError) -> IronResult<Response> {
        if let Some(_) = err.error.downcast::<NoRoute>() {
            Ok(json_response(status::NotFound,
                v1::CustomError {
                    short: "no_route".to_string(),
                    long: format!("The requested URL was not found on the server, path: {}", req.url)
                }
            ))
        } else {
            Err(err)
        }
    }
}
