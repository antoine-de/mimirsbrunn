use async_graphql::extensions::Tracing;
use async_graphql::*;
use async_graphql::{ErrorExtensions, FieldError};
use futures::stream::StreamExt;
// use places::coord::Coord;
use futures::pin_mut;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use tokio::io::AsyncReadExt;
use tracing::debug;

use crate::adapters::primary::bragi::autocomplete::{build_query, Coord, Filters};
use crate::adapters::primary::bragi::settings::QuerySettings;
use crate::domain::model::query_parameters::QueryParameters;
use crate::domain::ports::export::{Error as ExportError, Export};
use crate::domain::usecases::search_documents::SearchDocuments;

impl ErrorExtensions for ExportError {
    // lets define our base extensions
    fn extend(&self) -> FieldError {
        self.extend_with(|err, e| match err {
            &ExportError::DocumentRetrievalError { source } => e.set("reason", source.to_string()),
        })
    }
}

/*
 * The following is an attempt at turning coord into an async_graphql input type,
 * but, in the interest of time, i leave the end for later
#[derive(Debug, Serialize, Deserialize)]
struct InputCoord(Coord);

impl Display for InputCoord {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "(lat: {}, lon: {})", self.0.lat(), self.0.lon())
    }
}

impl Type for InputCoord {
}
impl InputType for InputCoord {
    fn parse(value: GraphqlValue) -> InputValueResult<Self> {
        match &value {
            GraphqlValue::Object(o) => {
                let lat = o.get("lat").ok_or(InputValueError::custom("missing lat"))?;
                let lat = lat
                    .into_json()
                    .unwrap()
                    .as_f64()
                    .ok_or(InputValueError::custom("lat is not f64"))?;
                let lon: f64 = o.get("lon").ok_or(InputValueError::custom("missing lon"));
                let lon = lon
                    .into_json()
                    .unwrap()
                    .as_f64()
                    .ok_or(InputValueError::custom("lat is not f64"))?;
                Ok(InputCoord(Coord::new(lon, lat)))
            }
            _ => Err(InputValueError::expected_type(value)),
        }
    }

    fn to_value(&self) -> GraphqlValue {
        let v = serde_json::to_value(self.0).expect("no pb").clone();
        GraphqlValue::from(v)
    }
}
*/

#[derive(Debug, Serialize, Deserialize, InputObject)]
#[serde(rename_all = "camelCase")]
struct InputFilters {
    lat: Option<f32>,
    lon: Option<f32>,
    shape: Option<String>,
    shape_scope: Option<Vec<String>>, // Here I merge shape and shape_scope together, (and I use str)
    datasets: Option<Vec<String>>,
    zone_types: Option<Vec<String>>,
    poi_types: Option<Vec<String>>,
}

impl From<InputFilters> for Filters {
    fn from(input: InputFilters) -> Self {
        Filters {
            // When option_zip_option becomes available: coord: input.lat.zip_with(input.lon, Coord::new),
            coord: match (input.lat, input.lon) {
                (Some(lat), Some(lon)) => Some(Coord::new(lat, lon)),
                _ => None,
            },
            shape: match (input.shape, input.shape_scope) {
                (Some(shape), Some(shape_scope)) => Some((shape, shape_scope)),
                _ => None,
            },
            datasets: input.datasets,
            zone_types: input.zone_types,
            poi_types: input.poi_types,
        }
    }
}
// I'm about here.... and it's late
// I need to identify the output type, and put it there instead of Index

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SearchResponseBody {
    pub docs: Vec<JsonValue>,
    pub docs_count: usize,
}

#[Object]
impl SearchResponseBody {
    async fn docs(&self) -> &Vec<JsonValue> {
        &self.docs
    }

    async fn docs_count(&self) -> &usize {
        &self.docs_count
    }
}

impl From<Vec<JsonValue>> for SearchResponseBody {
    fn from(values: Vec<JsonValue>) -> Self {
        SearchResponseBody {
            docs_count: values.len(),
            docs: values,
        }
    }
}

pub struct Query;

#[Object]
impl Query {
    // FIXME We need a query, even if it does nothing
    async fn no_op(&self, _context: &Context<'_>) -> FieldResult<Option<i32>> {
        Ok(None)
    }
}

pub struct Mutation;

#[Object]
impl Mutation {
    async fn forward_geocoder(
        &self,
        context: &Context<'_>,
        q: String,
        filters: InputFilters,
        settings: Upload,
    ) -> FieldResult<SearchResponseBody> {
        let usecase = get_usecase_from_context(context)?;

        // Read settings from uploaded file
        let settings = settings
            .value(context)
            .map_err(|err| to_err("extract settings from upload", "graphql", err.to_string()))?;

        let mut settings_content = String::new();
        let mut settings_file = tokio::fs::File::from_std(settings.content);
        settings_file
            .read_to_string(&mut settings_content)
            .await
            .map_err(|err| to_err("read settings from content", "graphql", err.to_string()))?;
        let settings = QuerySettings::new(&settings_content)
            .map_err(|err| to_err("invalid settings", "graphql", err.to_string()))?;

        let filters = Filters::from(filters);
        let query = build_query(&q, filters, &["fr"], &settings);

        let query_parameters = QueryParameters {
            dsl: query,
            containers: vec![String::from("munin_street")],
        };

        let stream = usecase.search_documents(query_parameters)?;

        pin_mut!(stream);

        let res = stream.collect::<Vec<JsonValue>>().await;
        let resp = SearchResponseBody::from(res);

        Ok(resp)
    }
}

pub type BragiSchema = Schema<Query, Mutation, EmptySubscription>;

pub fn bragi_schema<D: 'static>(usecase: SearchDocuments<D>) -> BragiSchema {
    Schema::build(Query, Mutation, EmptySubscription)
        .extension(Tracing)
        .data(usecase)
        .finish()
}

#[allow(clippy::borrowed_box)]
pub fn get_usecase_from_context<'ctx, D: 'static>(
    context: &'ctx Context,
) -> Result<&'ctx SearchDocuments<D>, async_graphql::Error>
where
{
    context.data::<SearchDocuments<D>>()
}

// #[cfg(test)]
// mod tests {
//     use super::mimir;
//     use super::*;
//     use serde_json::Value;
//     use std::convert::Infallible;
//     use warp::Filter;
//
//     // TODO How to create a function to return graphql_post, so we don't repeat it.
//     #[tokio::test]
//     async fn test_add_index() {
//         let mut service = mimir::MockMimirService::new();
//         service.expect_generate_index().times(1).returning(|name| {
//             Ok(mimir::Index {
//                 name: String::from(name),
//             })
//         });
//
//         let schema = schema(Box::new(service));
//
//         let graphql_post = async_graphql_warp::graphql(schema).and_then(
//             |(schema, request): (IndexSchema, async_graphql::Request)| async move {
//                 Ok::<_, Infallible>(async_graphql_warp::Response::from(
//                     schema.execute(request).await,
//                 ))
//             },
//         );
//         let query = r#" "mutation createIndex($index: IndexConfig!) { createIndex(index: $index) { name, status, docsCount } }" "#;
//         let variables = r#" { "name": "foo" }"#;
//         let body = format!(
//             r#"{{ "query": {query}, "variables": {{ "index": {variables} }} }}"#,
//             query = query,
//             variables = variables
//         );
//
//         let resp = warp::test::request()
//             .method("POST")
//             .body(body)
//             .reply(&graphql_post)
//             .await;
//
//         assert_eq!(resp.status(), 200);
//         let data = resp.into_body();
//         let v: Value = serde_json::from_slice(&data).expect("json");
//         let c: mimir::Index =
//             serde_json::from_value(v["data"]["createIndex"].to_owned()).expect("index");
//         assert_eq!(c.name, "foo");
//         assert_eq!(c.status, "open");
//         assert_eq!(c.docs_count, 0);
//     }
// }
//
fn to_err(
    context: impl AsRef<str>,
    reason: impl AsRef<str>,
    details: String,
) -> async_graphql::Error {
    let mut extensions = async_graphql::ErrorExtensionValues::default();
    extensions.set(reason, details);
    async_graphql::Error {
        message: context.as_ref().to_string(),
        extensions: Some(extensions),
    }
}
