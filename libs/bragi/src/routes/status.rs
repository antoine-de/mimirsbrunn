use crate::Context;
use actix_web::web::{Data, Json};
use serde::{Deserialize, Serialize};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Serialize, Deserialize, Debug)]
pub struct Status {
    pub version: String,
    pub es: String,
    pub status: String,
}

pub fn status(state: Data<Context>) -> Json<Status> {
    Json(Status {
        version: VERSION.to_string(),
        es: state.cnx_string.clone(),
        status: "good".to_string(),
    })
}
