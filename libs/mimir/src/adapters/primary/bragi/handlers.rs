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
use crate::domain::model::configuration::{root_doctype, root_doctype_dataset};
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
    let timeout = params.timeout;
    let es_indices_to_search_in = build_es_indices_to_search(&params);
    let filters = filters::Filters::from((params, geometry));
    let dsl = dsl::build_query(&q, filters.clone(), &["fr"], &settings);

    debug!("{}", serde_json::to_string(&dsl).unwrap());

    match client
        .search_documents(
            es_indices_to_search_in,
            Query::QueryDSL(dsl),
            filters.limit,
            timeout,
        )
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
        .explain_document(Query::QueryDSL(dsl), params.doc_id, params.doc_type)
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

    let es_indices_to_search_in = vec![
        root_doctype(Street::static_doc_type()),
        root_doctype(Addr::static_doc_type()),
    ];

    match client
        .search_documents(
            es_indices_to_search_in,
            Query::QueryDSL(dsl),
            params.limit,
            params.timeout,
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

fn build_es_indices_to_search(query: &ForwardGeocoderQuery) -> Vec<String> {
    // some specific types are requested,
    // let's search only for these types of objects
    if let Some(types) = &query.types {
        let mut indices = Vec::new();
        for doc_type in types.iter() {
            match doc_type {
                Type::House => indices.push(root_doctype(Addr::static_doc_type())),
                Type::Street => indices.push(root_doctype(Street::static_doc_type())),
                Type::Zone => indices.push(root_doctype(Admin::static_doc_type())),
                Type::Poi => {
                    let doc_type_str = Poi::static_doc_type();
                    // if some poi_dataset are specified
                    // we search for poi only in the corresponding es indices
                    if let Some(poi_datasets) = &query.poi_dataset {
                        for poi_dataset in poi_datasets.iter() {
                            indices.push(root_doctype_dataset(doc_type_str, poi_dataset));
                        }
                    } else {
                        // no poi_dataset specified
                        // we search in the global alias for all poi
                        indices.push(root_doctype(doc_type_str));
                    }
                }
                Type::StopArea => {
                    // if some pt_dataset are specified
                    // we search for stops only in the corresponding es indices
                    let doc_type_str = Stop::static_doc_type();
                    if let Some(pt_datasets) = &query.pt_dataset {
                        for pt_dataset in pt_datasets.iter() {
                            indices.push(root_doctype_dataset(doc_type_str, pt_dataset));
                        }
                    } else {
                        // no pt_dataset specified
                        // we search in the global alias for all stops
                        indices.push(root_doctype(doc_type_str));
                    }
                }
            }
        }
        indices
    }
    // no types specified, we search for all objects in all indices
    else {
        vec![
            root_doctype(Admin::static_doc_type()),
            root_doctype(Street::static_doc_type()),
            root_doctype(Addr::static_doc_type()),
            root_doctype(Stop::static_doc_type()),
            root_doctype(Poi::static_doc_type()),
        ]
    }
}
