use crate::Context;
use actix_web::{HttpRequest, HttpResponse, Responder};
use prometheus;
use prometheus::Encoder;

#[derive(Serialize, Deserialize, Debug)]
pub struct EndPoint {
    pub description: String,
}

pub fn metrics(_r: &HttpRequest<Context>) -> impl Responder {
    let encoder = prometheus::TextEncoder::new();
    let metric_familys = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_familys, &mut buffer).unwrap();
    HttpResponse::Ok()
        .content_type(encoder.format_type())
        .body(buffer)
}
