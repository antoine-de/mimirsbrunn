use crate::error::Error;
use crate::{State, Step, StepStatus};
use async_trait::async_trait;
use common::document::ContainerDocument;
use cucumber::{t, Steps};
use mimir2::adapters::primary::common::dsl::build_query;
use mimir2::adapters::primary::common::filters::Filters;
use mimir2::adapters::primary::common::settings::QuerySettings;
use mimir2::adapters::secondary::elasticsearch;
use mimir2::adapters::secondary::elasticsearch::{ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ};
use mimir2::domain::model::query::Query;
use mimir2::domain::ports::primary::search_documents::SearchDocuments;
use mimir2::domain::ports::secondary::remote::Remote;
use places::admin::Admin;
use std::path::PathBuf;

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.when_regex_async(
        "the user searches for \"(.*)\"",
        t!(|mut state, ctx| {
            let query = ctx.matches[1].clone();

            state
                .execute(Search::new(query))
                .await
                .expect("failed to index cosmogony file");

            state
        }),
    );

    steps.then_regex_async(
        "he finds \"(.*)\" as the first result",
        t!(|mut state, ctx| {
            let id = ctx.matches[1].clone();

            state
                .execute(HasDocument { id, max_rank: 1 })
                .await
                .expect("failed to index cosmogony file");

            state
        }),
    );

    steps
}

/// Perform a search in current Elasticsearch DB.
pub struct Search {
    query: String,
    results: Vec<serde_json::Value>,
}

impl Search {
    pub fn new(query: String) -> Self {
        Self {
            query,
            results: Vec::new(),
        }
    }
}

#[async_trait(?Send)]
impl Step for Search {
    async fn execute(&mut self, _state: &State) -> Result<StepStatus, Error> {
        // Build query settings
        let base_path = env!("CARGO_MANIFEST_DIR");

        let query_settings_file: PathBuf = [base_path, "config", "query", "settings.toml"]
            .iter()
            .collect();

        let query_settings = QuerySettings::new_from_file(query_settings_file)
            .await
            .expect("query settings");

        // Connect to elasticsearch
        let pool = elasticsearch::remote::connection_test_pool().await.unwrap();

        let client = pool
            .conn(ES_DEFAULT_TIMEOUT, ES_DEFAULT_VERSION_REQ)
            .await
            .unwrap();

        // Build ES query
        let dsl = build_query(&self.query, Filters::default(), &["fr"], &query_settings);

        // Fetch documents
        self.results = {
            client
                .search_documents(
                    vec![Admin::static_doc_type().to_string()],
                    Query::QueryDSL(dsl),
                )
                .await
                .unwrap()
        };

        Ok(StepStatus::Done)
    }
}

/// Check if given document is in the output.
///
/// It assumes that a Search has already been performed before.
pub struct HasDocument {
    id: String,
    max_rank: usize,
}

#[async_trait(?Send)]
impl Step for HasDocument {
    async fn execute(&mut self, state: &State) -> Result<StepStatus, Error> {
        let (search, _) = state
            .steps_for::<Search>()
            .next_back()
            .expect("the user must perform a search before checking results");

        let (rank, _) = (search.results.iter().enumerate())
            .find(|(_, doc)| self.id == doc["id"].as_str().unwrap())
            .expect("document was not found in search results");

        assert!(rank < self.max_rank);
        Ok(StepStatus::Done)
    }
}
