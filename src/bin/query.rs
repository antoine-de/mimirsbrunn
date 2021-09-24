use common::document::ContainerDocument;
use mimir2::{
    adapters::primary::common::settings::QuerySettings,
    adapters::primary::common::{dsl::build_query, filters::Filters},
    adapters::secondary::elasticsearch::remote::connection_test_pool,
    domain::model::query::Query,
    domain::ports::primary::search_documents::SearchDocuments,
    domain::ports::secondary::remote::Remote,
};
use places::admin::Admin;

#[tokio::main]
async fn main() {
    let args: Vec<String> = std::env::args().collect();
    let q = &args[1];

    let pool = connection_test_pool()
        .await
        .expect("Elasticsearch Connection Pool");
    let client = pool
        .conn()
        .await
        .expect("Elasticsearch Connection Established");

    let filters = Filters::default();

    let settings = QuerySettings::default();

    let dsl = build_query(q, filters, &["fr"], &settings);

    client
        .search_documents(
            vec![Admin::static_doc_type().to_string()],
            Query::QueryDSL(dsl),
        )
        .await
        .unwrap()
        .iter()
        .enumerate()
        .for_each(|(i, v): (_, &serde_json::Value)| {
            println!("{}: {} | {} | {}", i, v["id"], v["name"], v["label"]);
        });
}
