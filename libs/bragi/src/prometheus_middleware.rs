use actix_web::middleware::{Finished, Middleware, Started};
use actix_web::{HttpRequest, HttpResponse, Result};
use prometheus::{
    histogram_opts, labels, opts, register_counter_vec, register_gauge, register_histogram_vec,
};

lazy_static::lazy_static! {
    static ref HTTP_COUNTER: prometheus::CounterVec = register_counter_vec!(
        "bragi_http_requests_total",
        "Total number of HTTP requests made.",
        &["handler", "method", "status"]
    )
    .unwrap();
    static ref HTTP_REQ_HISTOGRAM: prometheus::HistogramVec = register_histogram_vec!(
        "bragi_http_request_duration_seconds",
        "The HTTP request latencies in seconds.",
        &["handler", "method"],
        prometheus::exponential_buckets(0.001, 1.5, 25).unwrap()
    )
    .unwrap();
    static ref HTTP_IN_FLIGHT: prometheus::Gauge = register_gauge!(
        "bragi_http_requests_in_flight",
        "current number of http request being served"
    )
    .unwrap();
}

pub struct PrometheusMiddleware;

impl Default for PrometheusMiddleware {
    fn default() -> Self {
        PrometheusMiddleware {}
    }
}

impl<S> Middleware<S> for PrometheusMiddleware {
    fn start(&self, req: &HttpRequest<S>) -> Result<Started> {
        HTTP_REQ_HISTOGRAM
            .get_metric_with(&labels! {
                "handler" => req.resource().name(),
                "method" => req.method().as_str(),
            })
            .map(|timer| {
                req.extensions_mut().insert(timer.start_timer());
            })
            .unwrap_or_else(|err| {
                error!("impossible to get HTTP_REQ_HISTOGRAM metrics";
                               "err" => err.to_string());
            });
        HTTP_IN_FLIGHT.inc();
        Ok(Started::Done)
    }

    fn finish(&self, req: &HttpRequest<S>, resp: &HttpResponse) -> Finished {
        HTTP_IN_FLIGHT.dec();

        let status = resp.status().to_string();
        HTTP_COUNTER
            .get_metric_with(&labels! {
                "handler" => req.resource().name(),
                "method" => req.method().as_str(),
                "status" => status.as_str(),
            })
            .map(|counter| counter.inc())
            .unwrap_or_else(|err| {
                error!("impossible to get HTTP_COUNTER metrics"; "err" => err.to_string());
            });
        Finished::Done
    }
}
