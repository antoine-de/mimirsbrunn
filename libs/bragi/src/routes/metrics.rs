use actix_web::{HttpResponse, Responder};
use prometheus;
use prometheus::Encoder;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct EndPoint {
    pub description: String,
}

pub fn metrics() -> impl Responder {
    let encoder = prometheus::TextEncoder::new();
    let metric_familys = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_familys, &mut buffer).unwrap();
    HttpResponse::Ok()
        .content_type(encoder.format_type())
        .body(buffer)
}
