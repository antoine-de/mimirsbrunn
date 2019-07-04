use crate::Context;
use actix_web::{Json, State};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct Status {
    pub version: String,
    pub es: String,
    pub status: String,
}

pub fn status(state: State<Context>) -> Json<Status> {
    Json(Status {
        version: env!("CARGO_PKG_VERSION").to_string(),
        es: state.cnx_string.clone(),
        status: "good".to_string(),
    })
}
