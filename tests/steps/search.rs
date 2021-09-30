use crate::error::Error;
use crate::state::{State, Step, StepStatus};
use async_trait::async_trait;
use common::document::ContainerDocument;
use cucumber::{t, StepContext, Steps};
use mimir2::adapters::primary::common::dsl::build_query;
use mimir2::adapters::primary::common::filters::Filters;
use mimir2::adapters::primary::common::settings::QuerySettings;
use mimir2::adapters::secondary::elasticsearch::ElasticsearchStorage;
use mimir2::domain::model::query::Query;
use mimir2::domain::ports::primary::search_documents::SearchDocuments;
use places::addr::Addr;
use places::admin::Admin;

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.when_regex_async(
        "the user searches for \"(.*)\"",
        t!(|mut state, ctx| {
            let query = ctx.matches[1].clone();

            state
                .execute(Search::new(query), &ctx)
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
                .execute(HasDocument { id, max_rank: 1 }, &ctx)
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
    async fn execute(&mut self, _state: &State, ctx: &StepContext) -> Result<StepStatus, Error> {
        let client: &ElasticsearchStorage = ctx.get().expect("could not get ES client");

        // Build ES query
        let dsl = build_query(
            &self.query,
            Filters::default(),
            &["fr"],
            &QuerySettings::default(),
        );

        // Fetch documents
        self.results = {
            client
                .search_documents(
                    vec![
                        Admin::static_doc_type().to_string(),
                        Addr::static_doc_type().to_string(),
                    ],
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
    async fn execute(&mut self, state: &State, _ctx: &StepContext) -> Result<StepStatus, Error> {
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
