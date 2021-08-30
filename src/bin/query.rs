use mimir2::{
    adapters::primary::bragi::autocomplete::{build_query, Filters},
    adapters::primary::bragi::settings::QuerySettings,
    adapters::secondary::elasticsearch::remote::connection_test_pool,
    domain::model::query::Query,
    domain::ports::primary::search_documents::SearchDocuments,
    domain::ports::secondary::remote::Remote,
};
use places::{admin::Admin, MimirObject};
use std::path::PathBuf;

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

    let mut query_settings_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    query_settings_file.push("config");
    query_settings_file.push("query");
    query_settings_file.push("settings.toml");
    let query_settings = QuerySettings::new_from_file(query_settings_file)
        .await
        .expect("query settings");

    let dsl = build_query(q, filters, &["fr"], &query_settings);

    client
        .search_documents(vec![String::from(Admin::doc_type())], Query::QueryDSL(dsl))
        .await
        .unwrap()
        .iter()
        .enumerate()
        .for_each(|(i, v): (_, &serde_json::Value)| {
            println!("{}: {} | {} | {}", i, v["id"], v["name"], v["label"]);
        });
}
