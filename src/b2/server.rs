use async_graphql::http::{playground_source, GraphQLPlaygroundConfig};
use clap::ArgMatches;
use http::StatusCode;
use snafu::{ResultExt, Snafu};
use std::convert::Infallible;
use std::marker::PhantomData;
use std::net::ToSocketAddrs;
use tracing::{info, instrument};
use tracing_bunyan_formatter::{BunyanFormattingLayer, JsonStorageLayer};
use tracing_log::LogTracer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::{EnvFilter, Registry};
use warp::{http::Response as HttpResponse, Filter, Rejection};

use super::settings::{Error as SettingsError, Settings as CSettings};
//use mimir::domain::ports::index::IndexServiceImpl;
//use mimir::domain::ports::remote::Remote;
//use mimir::domain::usecases::generate_index::GenerateIndex;
//use mimir::obj::Obj;
use mimir2::{
    adapters::primary::bragi::gql,
    adapters::secondary::elasticsearch::{
        self,
        internal::{IndexConfiguration, IndexMappings, IndexParameters, IndexSettings},
    },
    domain::model::query_parameters::QueryParameters,
    domain::ports::remote::Remote,
    domain::usecases::search_documents::{SearchDocuments, SearchDocumentsParameters},
    domain::usecases::UseCase,
};

#[derive(Debug, Snafu)]
pub enum Error {
    #[snafu(display("Could not get connection pool: {}", source))]
    Connection { source: Box<dyn std::error::Error> },

    #[snafu(display("Could not generate settings: {}", source))]
    Settings { source: SettingsError },

    #[snafu(display("Socket Addr Error {}", source))]
    SockAddr { source: std::io::Error },

    #[snafu(display("Addr Resolution Error {}", msg))]
    AddrResolution { msg: String },
}

#[allow(clippy::needless_lifetimes)]
pub async fn run<'a>(matches: &ArgMatches<'a>) -> Result<(), Error> {
    let settings = CSettings::new(matches).context(Settings)?;
    LogTracer::init().expect("Unable to setup log tracer!");

    // following code mostly from https://betterprogramming.pub/production-grade-logging-in-rust-applications-2c7fffd108a6
    let app_name = concat!(env!("CARGO_PKG_NAME"), "-", env!("CARGO_PKG_VERSION")).to_string();

    let file_appender = tracing_appender::rolling::daily(&settings.logging.path, "mimir.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);
    // let (non_blocking, _guard) = tracing_appender::non_blocking(std::io::stdout());

    let bunyan_formatting_layer = BunyanFormattingLayer::new(app_name, non_blocking);
    let subscriber = Registry::default()
        .with(EnvFilter::new("INFO"))
        .with(JsonStorageLayer)
        .with(bunyan_formatting_layer);
    tracing::subscriber::set_global_default(subscriber).expect("tracing subscriber global default");

    run_server(settings).await
}

#[instrument]
pub async fn run_server(settings: CSettings) -> Result<(), Error> {
    let pool = elasticsearch::remote::connection_pool()
        .await
        .map_err(|err| Error::Connection {
            source: Box::new(err),
        })?;

    let client = pool.conn().await.map_err(|err| Error::Connection {
        source: Box::new(err),
    })?;

    let service = SearchDocuments::new(Box::new(client));

    let schema = gql::bragi_schema(service);

    let graphql_post = async_graphql_warp::graphql(schema).and_then(
        |(schema, request): (gql::BragiSchema, async_graphql::Request)| async move {
            // let request_id = Uuid::new_v4();
            // let root_span = span!(parent: None, Level::INFO, "graphql request", %request_id);
            // let request = request.data(Tracing::default().parent_span(root_span));
            Ok::<_, Infallible>(async_graphql_warp::Response::from(
                schema.execute(request).await,
            ))
        },
    );

    let graphql_playground = warp::path("playground").and(warp::get()).map(|| {
        HttpResponse::builder()
            .header("content-type", "text/html")
            .body(playground_source(GraphQLPlaygroundConfig::new("/")))
    });

    let routes = graphql_playground
        .or(graphql_post)
        .recover(|err: Rejection| async move {
            if let Some(async_graphql_warp::BadRequest(err)) = err.find() {
                return Ok::<_, Infallible>(warp::reply::with_status(
                    err.to_string(),
                    StatusCode::BAD_REQUEST,
                ));
            }

            Ok(warp::reply::with_status(
                "INTERNAL_SERVER_ERROR".to_string(),
                StatusCode::INTERNAL_SERVER_ERROR,
            ))
        });

    let host = settings.service.host;
    let port = settings.service.port;
    let addr = (host.as_str(), port);
    let addr = addr
        .to_socket_addrs()
        .context(SockAddr)?
        .next()
        .ok_or(Error::AddrResolution {
            msg: String::from("Cannot resolve addr"),
        })?;

    info!("Serving stocks on {}", addr);
    warp::serve(routes).run(addr).await;

    Ok(())
}
