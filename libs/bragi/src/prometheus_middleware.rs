// shamelessly taken from https://github.com/nlopes/actix-web-prom
// we are unfortunatly not able to directly use this great crate,
// because  we want to use the name of the endpoint for retrocompatibility
// (and as a side effect we also added the 'in flight' queries (but for this we could have used the Registry))

use actix_service::{Service, Transform};
use actix_web::{
    dev::{Body, BodySize, MessageBody, ResponseBody, ServiceRequest, ServiceResponse},
    http::{Method, StatusCode},
    web::Bytes,
    Error,
};
use futures::future::{ok, FutureResult};
use futures::{Async, Future, Poll};
use prometheus::{self, Encoder, TextEncoder};
use std::marker::PhantomData;
use std::sync::Arc;
use std::time::SystemTime;

lazy_static::lazy_static! {
    static ref PATH_TO_NAME: std::collections::HashMap<&'static str, &'static str> = {
        let mut map = std::collections::HashMap::new();
        map.insert("/", "/");
        map.insert("/metrics", "metrics");
        map.insert("/status", "status");
        map.insert("/reverse", "reverse");
        map.insert("/autocomplete", "autocomplete");
        map
    };

    static ref FEATURES_ROUTE: &'static str = "features";
}

fn get_ressource_name(path: &str) -> String {
    // we can't use the ressource's name in the current actix version,
    // so we use an hardcoded associated table
    PATH_TO_NAME
        .get(path)
        .copied()
        .unwrap_or_else(|| {
            if path.starts_with("/features") {
                &FEATURES_ROUTE
            } else {
                ""
            }
        })
        .to_string()
}

lazy_static::lazy_static! {
    static ref HTTP_COUNTER: prometheus::CounterVec = prometheus::register_counter_vec!(
        "bragi_http_requests_total",
        "Total number of HTTP requests made.",
        &["handler", "method", "status"]
    )
    .unwrap();

    static ref HTTP_REQ_HISTOGRAM: prometheus::HistogramVec = prometheus::register_histogram_vec!(
        "bragi_http_request_duration_seconds",
        "The HTTP request latencies in seconds.",
        &["handler", "method"],
        prometheus::exponential_buckets(0.001, 1.5, 25).unwrap()
    )
    .unwrap();

    static ref HTTP_IN_FLIGHT: prometheus::Gauge = prometheus::register_gauge!(
        "bragi_http_requests_in_flight",
        "current number of http request being served"
    )
    .unwrap();
}

#[derive(Clone)]
#[must_use = "must be set up as middleware for actix-web"]
/// By default two metrics are tracked (this assumes the namespace `actix_web_prom`):
///
///   - `actix_web_prom_http_requests_total` (labels: endpoint, method, status): the total
///   number of HTTP requests handled by the actix HttpServer.
///
///   - `actix_web_prom_http_requests_duration_seconds` (labels: endpoint, method,
///    status): the request duration for all HTTP requests handled by the actix
///    HttpServer.
pub struct PrometheusMetrics {
    pub(crate) namespace: String,
    pub(crate) endpoint: String,
}

impl PrometheusMetrics {
    /// Create a new PrometheusMetrics. You set the namespace and the metrics endpoint
    /// through here.
    pub fn new(namespace: &str, endpoint: &str) -> Self {
        PrometheusMetrics {
            namespace: namespace.to_string(),
            endpoint: endpoint.to_string(),
        }
    }

    fn metrics(&self) -> String {
        let mut buffer = vec![];
        TextEncoder::new()
            .encode(&prometheus::gather(), &mut buffer)
            .unwrap();
        String::from_utf8(buffer).unwrap()
    }

    fn matches(&self, path: &str, method: &Method) -> bool {
        self.endpoint == path && method == Method::GET
    }

    fn update_metrics(
        &self,
        handler: &str,
        method: &Method,
        status: StatusCode,
        clock: SystemTime,
    ) {
        let method = method.to_string();
        let status = status.as_u16().to_string();

        if let Ok(elapsed) = clock.elapsed() {
            let duration =
                (elapsed.as_secs() as f64) + f64::from(elapsed.subsec_nanos()) / 1_000_000_000_f64;
            HTTP_REQ_HISTOGRAM
                .with_label_values(&[&handler, &method])
                .observe(duration);
        }

        HTTP_COUNTER
            .with_label_values(&[&handler, &method, &status])
            .inc();

        HTTP_IN_FLIGHT.dec();
    }
}

impl<S, B> Transform<S> for PrometheusMetrics
where
    B: MessageBody,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<StreamLog<B>>;
    type Error = Error;
    type InitError = ();
    type Transform = PrometheusMetricsMiddleware<S>;
    type Future = FutureResult<Self::Transform, Self::InitError>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(PrometheusMetricsMiddleware {
            service,
            inner: Arc::new(self.clone()),
        })
    }
}

#[doc(hidden)]
/// Middleware service for PrometheusMetrics
pub struct PrometheusMetricsMiddleware<S> {
    service: S,
    inner: Arc<PrometheusMetrics>,
}

impl<S, B> Service for PrometheusMetricsMiddleware<S>
where
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: MessageBody,
{
    type Request = ServiceRequest;
    type Response = ServiceResponse<StreamLog<B>>;
    type Error = S::Error;
    type Future = MetricsResponse<S, B>;

    fn poll_ready(&mut self) -> Poll<(), Self::Error> {
        self.service.poll_ready()
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        let clock = SystemTime::now();
        MetricsResponse {
            fut: self.service.call(req),
            clock: clock,
            inner: self.inner.clone(),
            _t: PhantomData,
        }
    }
}

#[doc(hidden)]
pub struct MetricsResponse<S, B>
where
    B: MessageBody,
    S: Service,
{
    fut: S::Future,
    clock: SystemTime,
    inner: Arc<PrometheusMetrics>,
    _t: PhantomData<(B,)>,
}

impl<S, B> Future for MetricsResponse<S, B>
where
    B: MessageBody,
    S: Service<Request = ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
{
    type Item = ServiceResponse<StreamLog<B>>;
    type Error = Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        HTTP_IN_FLIGHT.inc();
        let res = futures::try_ready!(self.fut.poll());

        let req = res.request();
        let inner = self.inner.clone();
        let method = req.method().clone();
        let path = req.path().to_string();
        let handler = get_ressource_name(&path);

        Ok(Async::Ready(res.map_body(move |mut head, mut body| {
            // We short circuit the response status and body to serve the endpoint
            // automagically. This way the user does not need to set the middleware *AND*
            // an endpoint to serve middleware results. The user is only required to set
            // the middleware and tell us what the endpoint should be.
            if inner.matches(&path, &method) {
                head.status = StatusCode::OK;
                body = ResponseBody::Other(Body::from_message(inner.metrics()));
            }
            ResponseBody::Body(StreamLog {
                body,
                size: 0,
                clock: self.clock,
                inner,
                status: head.status,
                handler,
                method,
            })
        })))
    }
}

#[doc(hidden)]
pub struct StreamLog<B> {
    body: ResponseBody<B>,
    size: usize,
    clock: SystemTime,
    inner: Arc<PrometheusMetrics>,
    status: StatusCode,
    handler: String,
    method: Method,
}

impl<B> Drop for StreamLog<B> {
    fn drop(&mut self) {
        // update the metrics for this request at the very end of responding
        self.inner
            .update_metrics(&self.handler, &self.method, self.status, self.clock);
    }
}

impl<B: MessageBody> MessageBody for StreamLog<B> {
    fn size(&self) -> BodySize {
        self.body.size()
    }

    fn poll_next(&mut self) -> Poll<Option<Bytes>, Error> {
        match self.body.poll_next()? {
            Async::Ready(Some(chunk)) => {
                self.size += chunk.len();
                Ok(Async::Ready(Some(chunk)))
            }
            val => Ok(val),
        }
    }
}
