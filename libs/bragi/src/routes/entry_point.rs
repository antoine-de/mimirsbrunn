use crate::Context;
use actix_web::{HttpRequest, Json, Responder};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct EndPoint {
    pub description: String,
}

pub fn entry_point(_r: &HttpRequest<Context>) -> impl Responder {
    Json(EndPoint {
        description: "autocomplete service".to_owned(),
    })
}
