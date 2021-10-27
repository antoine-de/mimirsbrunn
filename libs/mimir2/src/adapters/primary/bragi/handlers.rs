use geojson::Geometry;
use serde::Serialize;
use tracing::{debug, instrument};
use warp::http::StatusCode;
use warp::reply::{json, with_status};

use crate::adapters::primary::bragi::api::ForwardGeocoderExplainQuery;
use crate::adapters::primary::{
    bragi::api::{
        BragiStatus, ElasticsearchStatus, ForwardGeocoderQuery, MimirStatus, ReverseGeocoderQuery,
        StatusResponseBody, Type,
    },
    common::{
        dsl, filters, geocoding::Feature, geocoding::FromWithLang, geocoding::GeocodeJsonResponse,
        settings,
    },
};
use crate::domain::model::query::Query;
use crate::domain::ports::primary::explain_query::ExplainDocument;
use crate::domain::ports::primary::search_documents::SearchDocuments;
use crate::domain::ports::primary::status::Status;
use common::document::ContainerDocument;
use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street, Place};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[instrument(skip(client, settings))]
pub async fn forward_geocoder<S>(
    params: ForwardGeocoderQuery,
    geometry: Option<Geometry>,
    client: S,
    settings: settings::QuerySettings,
) -> Result<impl warp::Reply, warp::Rejection>
where
    S: SearchDocuments,
    S::Document: Serialize + Into<serde_json::Value>,
{
    let q = params.q.clone();
    let search_types = types_to_indices(&params.types);
    let filters = filters::Filters::from((params, geometry));
    let dsl = dsl::build_query(&q, filters, &["fr"], &settings);

    debug!("{}", serde_json::to_string(&dsl).unwrap());

    match client
        .search_documents(search_types, Query::QueryDSL(dsl))
        .await
    {
        Ok(res) => {
            let places: Result<Vec<Place>, serde_json::Error> = res
                .into_iter()
                .map(|json| serde_json::from_value::<Place>(json.into()))
                .collect();

            match places {
                Ok(places) => {
                    let features = places
                        .into_iter()
                        .map(|p| Feature::from_with_lang(p, None)) // FIXME lang: None
                        .collect();
                    let resp = GeocodeJsonResponse::new(q, features);
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

#[instrument(skip(client, settings))]
pub async fn forward_geocoder_explain<S>(
    params: ForwardGeocoderExplainQuery,
    geometry: Option<Geometry>,
    client: S,
    settings: settings::QuerySettings,
) -> Result<impl warp::Reply, warp::Rejection>
where
    S: ExplainDocument,
    S::Document: Serialize + Into<serde_json::Value>,
{
    let q = params.query.q.clone();
    let filters = filters::Filters::from((params.query, geometry));
    let dsl = dsl::build_query(&q, filters, &["fr"], &settings);

    debug!("{}", serde_json::to_string(&dsl).unwrap());

    match client
        .explain_document(Query::QueryDSL(dsl), params.id, params.doc_type)
        .await
    {
        Ok(res) => Ok(with_status(json(&res), StatusCode::OK)),
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
    S::Document: Serialize + Into<serde_json::Value>,
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
            let places = res
                .into_iter()
                .map(|json| serde_json::from_value::<Place>(json.into()).unwrap())
                .collect();

            let resp = GeocodeJsonResponse::from_with_lang(places, None);
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
                mimir: MimirStatus {
                    version: res.version,
                },
                elasticsearch: ElasticsearchStatus {
                    version: res.storage.version,
                    health: res.storage.health.to_string(),
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

// This translate the search types requested in the query into index types to search for.
// If no type were specified, we search all indices.
fn types_to_indices(types: &Option<Vec<Type>>) -> Vec<String> {
    if let Some(ts) = types {
        ts.iter().map(|t| t.as_index_type().to_string()).collect()
    } else {
        vec![
            String::from(Admin::static_doc_type()),
            String::from(Street::static_doc_type()),
            String::from(Addr::static_doc_type()),
            String::from(Stop::static_doc_type()),
            String::from(Poi::static_doc_type()),
        ]
    }
}
