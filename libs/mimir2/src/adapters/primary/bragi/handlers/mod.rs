use geojson::Geometry;
use serde::Serialize;
use warp::http::StatusCode;
use warp::reply::{json, with_status};

use crate::adapters::primary::{
    bragi::api::{
        BragiStatus, ElasticsearchStatus, ForwardGeocoderQuery, ReverseGeocoderQuery,
        SearchResponseBody, StatusResponseBody,
    },
    common::{dsl, filters, settings},
};
use crate::domain::model::query::Query;
use crate::domain::ports::primary::search_documents::SearchDocuments;
use crate::domain::ports::primary::status::Status;
use common::document::ContainerDocument;
use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub async fn forward_geocoder<S>(
    params: ForwardGeocoderQuery,
    geometry: Option<Geometry>,
    client: S,
    settings: settings::QuerySettings,
) -> Result<impl warp::Reply, warp::Rejection>
where
    S: SearchDocuments,
    S::Document: Serialize,
{
    let q = params.q.clone();
    let filters = filters::Filters::from((params, geometry));

    let dsl = dsl::build_query(&q, filters, &["fr"], &settings);

    match client
        .search_documents(
            vec![
                String::from(Admin::static_doc_type()),
                String::from(Street::static_doc_type()),
                String::from(Addr::static_doc_type()),
                String::from(Stop::static_doc_type()),
                String::from(Poi::static_doc_type()),
            ],
            Query::QueryDSL(dsl),
        )
        .await
    {
        Ok(res) => {
            let resp = SearchResponseBody::from(res);
            Ok(with_status(json(&resp), StatusCode::OK))
        }
        Err(err) => Ok(with_status(
            json(&format!(
                "Error while searching {}: {}",
                &q,
                err.to_string()
            )),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn reverse_geocoder<S>(
    params: ReverseGeocoderQuery,
    client: S,
    settings: settings::QuerySettings,
) -> Result<impl warp::Reply, warp::Rejection>
where
    S: SearchDocuments,
    S::Document: Serialize,
{
    let distance = format!("{}m", settings.reverse_query.radius);
    let dsl = dsl::build_reverse_query(&distance, params.lat, params.lon);

    match client
        .search_documents(
            vec![
                String::from(Street::static_doc_type()),
                String::from(Addr::static_doc_type()),
            ],
            Query::QueryDSL(dsl),
        )
        .await
    {
        Ok(res) => {
            let resp = SearchResponseBody::from(res);
            Ok(with_status(json(&resp), StatusCode::OK))
        }
        Err(err) => Ok(with_status(
            json(&format!(
                "Error while reverse searching: {}",
                err.to_string()
            )),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}

pub async fn status<S>(client: S, url: String) -> Result<impl warp::Reply, warp::Rejection>
where
    S: Status,
{
    match client.status().await {
        Ok(res) => {
            let resp = StatusResponseBody {
                bragi: BragiStatus {
                    version: VERSION.to_string(),
                },
                elasticsearch: ElasticsearchStatus {
                    version: res.version,
                    health: res.health.to_string(),
                    url,
                },
            };
            Ok(with_status(json(&resp), StatusCode::OK))
        }
        Err(err) => Ok(with_status(
            json(&format!("Error while querying status: {}", err.to_string())),
            StatusCode::INTERNAL_SERVER_ERROR,
        )),
    }
}
