use async_graphql::extensions::Tracing;
use async_graphql::*;
use async_graphql::{ErrorExtensions, FieldError};
use futures::stream;
use serde::{Deserialize, Serialize};
use tokio::io::AsyncReadExt;
use tracing::debug;

use crate::domain::model::configuration::Configuration as ModelConfiguration;
use crate::domain::model::document::Document;
use crate::domain::model::index::Index as ModelIndex;
use crate::domain::ports::index::Error as IndexServiceError;
use crate::domain::usecases::generate_index::GenerateIndexParam;
use crate::domain::usecases::UseCase;
use crate::obj::Obj;

impl ErrorExtensions for IndexServiceError {
    // lets define our base extensions
    fn extend(&self) -> FieldError {
        self.extend_with(|err, e| match err {
            IndexServiceError::IndexCreation { .. } => e.set("reason", "Cannot create index"),
            IndexServiceError::IndexPublication { .. } => e.set("reason", "Cannot publish index"),
            IndexServiceError::StorageConnection { .. } => {
                e.set("reason", "Cannot connect to storage")
            }
            IndexServiceError::DocumentStreamInsertion { .. } => {
                e.set("reason", "Cannot insert documents")
            }
        })
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    pub name: String,
    pub status: String,
    pub docs_count: i32,
}

#[Object]
impl Index {
    async fn name(&self) -> &String {
        &self.name
    }

    async fn status(&self) -> &String {
        &self.status
    }

    async fn docs_count(&self) -> &i32 {
        &self.docs_count
    }
}

// FIXME The model index does not carry enough information.
impl From<ModelIndex> for Index {
    fn from(index: ModelIndex) -> Self {
        let ModelIndex { name, .. } = index;

        Index {
            name,
            status: String::from("All good"),
            docs_count: 0,
        }
    }
}

pub struct Query;

#[Object]
impl Query {
    async fn forward_geocoder(
        &self,
        context: &Context<'_>,
        q: String,
    ) -> FieldResult<Option<Index>> {
        Ok(None)
    }
}

pub type IndexSchema = Schema<Query, EmptyMutation, EmptySubscription>;

pub fn schema<T: Document + 'static>(
    usecase: Box<dyn UseCase<Res = ModelIndex, Param = GenerateIndexParam<T>> + Send + Sync>,
    doc_type: String,
) -> IndexSchema {
    Schema::build(Query, Mutation, EmptySubscription)
        .extension(Tracing)
        .data(usecase)
        .data(doc_type)
        .finish()
}

#[allow(clippy::borrowed_box)]
pub fn get_usecase_from_context<'ctx, T: Document>(
    context: &'ctx Context,
) -> Result<
    &'ctx Box<dyn UseCase<Res = ModelIndex, Param = GenerateIndexParam<T>> + Send + Sync>,
    async_graphql::Error,
>
where
{
    context
        .data::<Box<dyn UseCase<Res = ModelIndex, Param = GenerateIndexParam<T>> + Send + Sync>>()
}

#[allow(clippy::borrowed_box)]
pub fn get_doc_type_from_context<'ctx, T: Document + 'static>(
    context: &'ctx Context,
) -> Result<&'ctx String, async_graphql::Error>
where
{
    context.data::<String>()
}

#[derive(Debug, Serialize, Deserialize, InputObject)]
pub struct IndexParameters {
    pub timeout: String,
    pub wait_for_active_shards: String,
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
