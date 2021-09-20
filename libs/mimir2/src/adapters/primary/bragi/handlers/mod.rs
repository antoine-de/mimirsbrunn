use serde::{Deserialize, Serialize};
use warp::http::StatusCode;
use warp::reply::Reply;
use warp::reply::{json, with_status};

use crate::adapters::primary::{
    bragi::api::{InputQuery, SearchResponseBody},
    common::{dsl, filters, settings},
};
use crate::domain::model::query::Query;
use crate::domain::ports::primary::search_documents::SearchDocuments;
use common::document::ContainerDocument;
use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street};

pub async fn forward_geocoder<S>(
    params: InputQuery,
    client: S,
    settings: settings::QuerySettings,
) -> Result<warp::reply::Response, warp::Rejection>
where
    S: SearchDocuments,
    for<'de> <S as SearchDocuments>::Document: Deserialize<'de> + Serialize,
{
    let q = params.q.clone();
    let filters = filters::Filters::from(params);

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
            // Ok(with_status(json(&resp), StatusCode::OK).into())
            Ok(with_status(json(&resp), StatusCode::OK).into_response())
        }
        Err(err) => Ok(with_status(
            json(&format!(
                "Error while searching {}: {}",
                &q,
                err.to_string()
            )),
            StatusCode::INTERNAL_SERVER_ERROR,
        )
        .into_response()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::adapters::primary::common::settings::QuerySettings;
    use crate::domain::ports::primary::search_documents::MockSearchDocuments;

    // This unit test doesn't test much.... just that when we use the forward_geocoder handler,
    // then the domain's primary port SearchDocument::search_document is called, which is not
    // very useful...  But then, its a start, and we should have more tests in the dsl module to
    // make sure the query dsl is correctly constructed.
    #[tokio::test]
    async fn should_call_primary_port() {
        let query = InputQuery {
            q: String::from("paris"),
            ..Default::default()
        };
        let mut mock = MockSearchDocuments::new();
        mock.expect_search_documents()
            .times(1)
            .returning(|_, _| Ok(vec![]));
        let settings = QuerySettings::default();

        let resp = forward_geocoder(query, mock, settings).await.unwrap();
        assert_eq!(resp.status(), warp::http::status::StatusCode::OK);
    }
}
