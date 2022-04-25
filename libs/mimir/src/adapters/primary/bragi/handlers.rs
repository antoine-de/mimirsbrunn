use std::time::Duration;

use geo::algorithm::haversine_distance::HaversineDistance;
use geojson::Geometry;
use serde::{Deserialize, Serialize};
use tracing::instrument;
use warp::{
    http::StatusCode,
    reject::Reject,
    reply::{json, with_status},
};

use crate::{
    adapters::primary::{
        bragi::{
            api::{
                BragiStatus, ElasticsearchStatus, FeaturesQuery, ForwardGeocoderExplainQuery,
                ForwardGeocoderQuery, MimirStatus, ReverseGeocoderQuery, StatusResponseBody, Type,
            },
            prometheus_handler,
        },
        common::{
            coord, dsl,
            dsl::QueryType,
            filters,
            geocoding::{Feature, FromWithLang, GeocodeJsonResponse},
            settings,
        },
    },
    domain::{
        model::{
            configuration::{root_doctype, root_doctype_dataset},
            query::Query,
        },
        ports::primary::{
            explain_query::ExplainDocument, get_documents::GetDocuments,
            search_documents::SearchDocuments, status::Status,
        },
    },
};
use common::document::ContainerDocument;
use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street, Place};

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[cfg(feature = "metrics")]
lazy_static::lazy_static! {
    static ref ES_REQ_HISTOGRAM: prometheus::HistogramVec = prometheus::register_histogram_vec!(
        "bragi_elasticsearch_request_duration_seconds",
        "The elasticsearch request latencies in seconds.",
        &["search_type"],
        prometheus::exponential_buckets(0.001, 1.5, 25).unwrap()
    )
    .unwrap();
}

#[derive(Deserialize, Serialize, Debug, PartialEq)]
pub enum InternalErrorReason {
    ElasticSearchError,
    SerializationError,
    ObjectNotFoundError,
    StatusError,
}

#[derive(Deserialize, Serialize, Debug)]
pub struct InternalError {
    pub reason: InternalErrorReason,
    pub info: String,
}

impl Reject for InternalError {}

pub fn build_feature(
    places: Vec<places::Place>,
    query_coord: Option<&coord::Coord>,
    lang: Option<&str>,
) -> Vec<Feature> {
    places
        .into_iter()
        .map(|mut p| {
            if let Some(coord) = query_coord {
                let geo_point = geo::Point::new(coord.lon as f64, coord.lat as f64);
                let pp: geo::Point<f64> = geo::Point::new(p.coord().lon(), p.coord().lat());
                let distance = geo_point.haversine_distance(&pp) as u32;
                p.set_distance(distance);
            }
            Feature::from_with_lang(p, lang)
        })
        .collect()
}

#[instrument(skip(client, settings))]
pub async fn forward_geocoder<S>(
    params: ForwardGeocoderQuery,
    geometry: Option<Geometry>,
    client: S,
    settings: settings::QuerySettings,
    timeout: Duration,
) -> Result<impl warp::Reply, warp::Rejection>
where
    S: SearchDocuments,
    S::Document: Serialize + Into<serde_json::Value>,
{
    let q = params.q.clone();
    let timeout = params.timeout.unwrap_or(timeout);
    let es_indices_to_search_in =
        build_es_indices_to_search(&params.types, &params.pt_dataset, &params.poi_dataset);
    let lang = params.lang.clone();
    let filters = filters::Filters::from((params, geometry));
    let excludes = ["boundary".to_string()];

    for query_type in [QueryType::PREFIX, QueryType::FUZZY] {
        let dsl_query = dsl::build_query(
            &q,
            &filters,
            lang.as_str(),
            &settings,
            query_type,
            Option::Some(&excludes),
        );

        tracing::trace!(
            query_type = ?query_type,
            indices = ?es_indices_to_search_in,
            query = tracing::field::display(dsl_query.to_string()),
            "Query ES",
        );

        #[cfg(feature = "metrics")]
        let timer = ES_REQ_HISTOGRAM
            .get_metric_with_label_values(&[query_type.as_str()])
            .map(|h| h.start_timer())
            .map_err(|err| {
                tracing::error_span!(
                    "impossible to get ES_REQ_HISTOGRAM metrics",
                    err = err.to_string().as_str()
                )
            })
            .ok();

        let res = client
            .search_documents(
                es_indices_to_search_in.clone(),
                Query::QueryDSL(dsl_query),
                filters.limit,
                Some(timeout),
            )
            .await;

        #[cfg(feature = "metrics")]
        if let Some(timer) = timer {
            timer.observe_duration();
        }

        let places: Vec<Place> = res
            .map_err(|err| {
                warp::reject::custom(InternalError {
                    reason: InternalErrorReason::ElasticSearchError,
                    info: err.to_string(),
                })
            })?
            .into_iter()
            .map(|json| serde_json::from_value::<Place>(json.into()))
            .collect::<Result<_, _>>()
            .map_err(|err| {
                warp::reject::custom(InternalError {
                    reason: InternalErrorReason::SerializationError,
                    info: err.to_string(),
                })
            })?;

        if !places.is_empty() {
            let features = build_feature(places, filters.coord.as_ref(), Some(lang.as_str()));
            let resp = GeocodeJsonResponse::new(q, features);
            return Ok(with_status(json(&resp), StatusCode::OK));
        }
    }

    Ok(with_status(
        json(&GeocodeJsonResponse::new(q, vec![])),
        StatusCode::OK,
    ))
}

#[instrument(skip(client, settings))]
pub async fn forward_geocoder_explain<S>(
    params: ForwardGeocoderExplainQuery,
    geometry: Option<Geometry>,
    client: S,
    settings: settings::QuerySettings,
    timeout: Duration,
) -> Result<impl warp::Reply, warp::Rejection>
where
    S: ExplainDocument,
    S::Document: Serialize + Into<serde_json::Value>,
{
    let doc_id = params.doc_id.clone();
    let doc_type = params.doc_type.clone();
    let q = params.q.clone();
    let lang = params.lang.clone();

    let filters = filters::Filters::from((params.into(), geometry));
    let dsl = dsl::build_query(
        &q,
        &filters,
        lang.as_str(),
        &settings,
        QueryType::PREFIX,
        None,
    );

    match client
        .explain_document(Query::QueryDSL(dsl), doc_id, doc_type)
        .await
    {
        Ok(res) => Ok(with_status(json(&res), StatusCode::OK)),
        Err(err) => Err(warp::reject::custom(InternalError {
            reason: InternalErrorReason::ElasticSearchError,
            info: err.to_string(),
        })),
    }
}

pub async fn reverse_geocoder<S>(
    params: ReverseGeocoderQuery,
    client: S,
    settings: settings::QuerySettings,
    timeout: Duration,
) -> Result<impl warp::Reply, warp::Rejection>
where
    S: SearchDocuments,
    S::Document: Serialize + Into<serde_json::Value>,
{
    let timeout = params.timeout.unwrap_or(timeout);
    let distance = format!("{}m", settings.reverse_query.radius);
    let dsl = dsl::build_reverse_query(&distance, params.lat, params.lon);

    let es_indices_to_search_in = vec![
        root_doctype(Street::static_doc_type()),
        root_doctype(Addr::static_doc_type()),
    ];

    tracing::trace!(
        "Searching in indexes {:?} with query {}",
        es_indices_to_search_in,
        serde_json::to_string_pretty(&dsl).unwrap()
    );

    match client
        .search_documents(
            es_indices_to_search_in,
            Query::QueryDSL(dsl),
            params.limit,
            Some(timeout),
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
        Err(err) => Err(warp::reject::custom(InternalError {
            reason: InternalErrorReason::ElasticSearchError,
            info: err.to_string(),
        })),
    }
}

pub async fn features<S>(
    doc_id: String,
    params: FeaturesQuery,
    client: S,
    timeout: Duration,
) -> Result<impl warp::Reply, warp::Rejection>
where
    S: GetDocuments,
    S::Document: Serialize + Into<serde_json::Value>,
{
    let timeout = params.timeout.unwrap_or(timeout);
    let es_indices_to_search_in =
        build_es_indices_to_search(&None, &params.pt_dataset, &params.poi_dataset);
    let dsl = dsl::build_features_query(&es_indices_to_search_in, &doc_id);

    tracing::trace!(
        "Searching in indexes {:?} with query {}",
        es_indices_to_search_in,
        serde_json::to_string_pretty(&dsl).unwrap()
    );

    match client
        .get_documents_by_id(Query::QueryDSL(dsl), Some(timeout))
        .await
    {
        Ok(res) => {
            let places: Result<Vec<Place>, serde_json::Error> = res
                .into_iter()
                .map(|json| serde_json::from_value::<Place>(json.into()))
                .collect();

            match places {
                Ok(places) if places.is_empty() => Err(warp::reject::custom(InternalError {
                    reason: InternalErrorReason::ObjectNotFoundError,
                    info: "Unable to find object".to_string(),
                })),
                Ok(places) => {
                    let features: Vec<Feature> = places
                        .into_iter()
                        .map(|p| Feature::from_with_lang(p, None)) // FIXME lang: None
                        .collect();
                    let resp = GeocodeJsonResponse::new("".to_string(), features);
                    Ok(with_status(json(&resp), StatusCode::OK))
                }
                Err(err) => Err(warp::reject::custom(InternalError {
                    reason: InternalErrorReason::SerializationError,
                    info: err.to_string(),
                })),
            }
        }
        Err(err) => Err(warp::reject::custom(InternalError {
            reason: InternalErrorReason::ElasticSearchError,
            info: err.to_string(),
        })),
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
        Err(err) => Err(warp::reject::custom(InternalError {
            reason: InternalErrorReason::StatusError,
            info: err.to_string(),
        })),
    }
}

pub async fn metrics() -> Result<impl warp::Reply, warp::Rejection> {
    let reply = warp::reply::with_header(
        prometheus_handler::metrics(),
        "content-type",
        "text/plain; charset=utf-8",
    );
    Ok(reply)
}

pub fn build_es_indices_to_search(
    types: &Option<Vec<Type>>,
    pt_dataset: &Option<Vec<String>>,
    poi_dataset: &Option<Vec<String>>,
) -> Vec<String> {
    // some specific types are requested,
    // let's search only for these types of objects
    if let Some(types) = types {
        let mut indices = Vec::new();
        for doc_type in types.iter() {
            match doc_type {
                Type::House => indices.push(root_doctype(Addr::static_doc_type())),
                Type::Street => indices.push(root_doctype(Street::static_doc_type())),
                Type::Zone | Type::City => indices.push(root_doctype(Admin::static_doc_type())),
                Type::Poi => {
                    let doc_type_str = Poi::static_doc_type();
                    // if some poi_dataset are specified
                    // we search for poi only in the corresponding es indices
                    if let Some(poi_datasets) = poi_dataset {
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
                    if let Some(pt_datasets) = pt_dataset {
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
    } else {
        let mut indices = vec![
            root_doctype(Addr::static_doc_type()),
            root_doctype(Street::static_doc_type()),
            root_doctype(Admin::static_doc_type()),
        ];
        if let Some(pt_datasets) = pt_dataset {
            let doc_type_str = Stop::static_doc_type();
            for pt_dataset in pt_datasets.iter() {
                indices.push(root_doctype_dataset(doc_type_str, pt_dataset));
            }
        }
        if let Some(poi_datasets) = poi_dataset {
            let doc_type_str = Poi::static_doc_type();
            for poi_dataset in poi_datasets.iter() {
                indices.push(root_doctype_dataset(doc_type_str, poi_dataset));
            }
        } else {
            indices.push(root_doctype(Poi::static_doc_type()))
        }
        indices
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::primary::bragi::routes::forward_geocoder_get;

    async fn indices_builder(query: &str) -> Vec<String> {
        let filter = forward_geocoder_get();
        let params = warp::test::request()
            .path(query)
            .filter(&filter)
            .await
            .unwrap();
        build_es_indices_to_search(&params.0.types, &params.0.pt_dataset, &params.0.poi_dataset)
    }

    // no dataset and no types
    #[tokio::test]
    #[should_panic]
    async fn no_dataset_no_type() {
        let es_indices = indices_builder("/api/v1/autocomplete?q=Bob").await;
        assert_eq!(es_indices, ["",]);
    }

    // no dataset + type public_transport:stop_area only
    #[tokio::test]
    #[should_panic]
    async fn no_dataset_with_type_sa() {
        let es_indices =
            indices_builder("/api/v1/autocomplete?q=Bob&type[]=public_transport:stop_area").await;
        assert_eq!(es_indices, [""]);
    }

    // no dataset + types poi, city, street, house
    #[tokio::test]
    #[should_panic]
    async fn no_dataset_all_types_but_sa() {
        let es_indices = indices_builder(
            "/api/v1/autocomplete?q=Bob&type[]=poi&type[]=city&type[]=street&type[]=house",
        )
        .await;
        assert_eq!(es_indices, ["",]);
    }

    // no dataset + types poi, city, street, house and public_transport:stop_area
    #[tokio::test]
    #[should_panic]
    async fn no_dataset_all_types() {
        let es_indices = indices_builder(
            "/api/v1/autocomplete?q=Bob&type[]=poi&type[]=city&type[]=street&type[]=house&type[]=public_transport:stop_area",
        )
            .await;
        assert_eq!(es_indices, ["",]);
    }

    // dataset fr + no type
    #[tokio::test]
    #[should_panic]
    async fn fr_dataset_no_type() {
        let es_indices = indices_builder("/api/v1/autocomplete?q=Bob&pt_dataset[]=fr").await;
        assert_eq!(es_indices, ["",]);
    }

    // dataset fr + type public_transport:stop_area only
    #[tokio::test]
    #[should_panic]
    async fn fr_pt_dataset_with_type_sa() {
        let es_indices = indices_builder(
            "/api/v1/autocomplete?q=Bob&pt_dataset[]=fr&type[]=public_transport:stop_area",
        )
        .await;
        assert_eq!(es_indices, [""]);
    }

    // no dataset + types poi, city, street, house
    #[tokio::test]
    #[should_panic]
    async fn fr_dataset_all_types_but_sa() {
        let es_indices = indices_builder(
            "/api/v1/autocomplete?q=Bob&pt_dataset[]=fr&type[]=poi&type[]=city&type[]=street&type[]=house",
        )
            .await;
        assert_eq!(es_indices, ["",]);
    }

    // dataset fr + types poi, city, street, house and public_transport:stop_area
    #[tokio::test]
    #[should_panic]
    async fn fr_dataset_all_types() {
        let es_indices = indices_builder("/api/v1/autocomplete?q=Bob&pt_dataset[]=fr&type[]=poi&type[]=city&type[]=street&type[]=house&type[]=public_transport:stop_area").await;
        assert_eq!(es_indices, ["",]);
    }

    // dataset fr + poi_dataset mti + types poi, city, street, house
    #[tokio::test]
    #[should_panic]
    async fn fr_dataset_mti_poi_dataset_all_types_but_sa() {
        let es_indices = indices_builder("/api/v1/autocomplete?q=Bob&pt_dataset[]=fr&poi_dataset[]=mti&type[]=poi&type[]=city&type[]=street&type[]=house").await;
        assert_eq!(es_indices, ["",]);
    }
}
