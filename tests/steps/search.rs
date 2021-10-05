use crate::error::Error;
use crate::state::{State, Step, StepStatus};
use async_trait::async_trait;
use cucumber::{t, StepContext, Steps};
use mimir2::{
    adapters::{
        primary::{
            common::dsl::build_query, common::filters::Filters, common::settings::QuerySettings,
        },
        secondary::elasticsearch::ElasticsearchStorage,
    },
    domain::{model::query::Query, ports::primary::search_documents::SearchDocuments},
};

pub fn steps() -> Steps<State> {
    let mut steps: Steps<State> = Steps::new();

    steps.when_regex_async(
        "the user searches \"(.*)\" for \"(.*)\"",
        t!(|mut state, ctx| {
            let domain: Vec<String> = ctx.matches[1]
                .clone()
                .split(", ")
                .map(|s| s.to_string())
                .collect();

            let query = ctx.matches[2].clone();

            state
                .execute(Search::new(domain, query), &ctx)
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
    domain: Vec<String>,
    query: String,
    results: Vec<serde_json::Value>,
}

impl Search {
    pub fn new(domain: Vec<String>, query: String) -> Self {
        Self {
            domain,
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
                .search_documents(self.domain.clone(), Query::QueryDSL(dsl))
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

        // println!("{:?}", search.results);

        let (rank, _) = (search.results.iter().enumerate())
            .find(|(_, doc)| self.id == doc["id"].as_str().unwrap())
            .expect("document was not found in search results");

        assert!(rank < self.max_rank);
        Ok(StepStatus::Done)
    }
}
