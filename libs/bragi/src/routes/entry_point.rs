use actix_web::web::Json;
use actix_web::Responder;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct EndPoint {
    pub description: String,
}

pub fn entry_point() -> impl Responder {
    Json(EndPoint {
        description: "autocomplete service".to_owned(),
    })
}
