use cucumber::async_trait;
use mimir2::adapters::primary::bragi::settings::QuerySettings;
use std::convert::Infallible;
use std::path::PathBuf;

pub struct MyWorld {
    query_settings: QuerySettings,
    search_result: Vec<serde_json::Value>,
}

#[async_trait(?Send)]
impl cucumber::World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        let mut query_settings_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        query_settings_file.push("config/query_settings.toml");
        let query_settings = QuerySettings::new_from_file(query_settings_file)
            .await
            .expect("query settings");
        Ok(Self {
            query_settings,
            search_result: Vec::new(),
        })
    }
}

mod example_steps {
    use cucumber::{t, Steps};
    // use failure::format_err;
    use mimir2::{
        adapters::primary::bragi::autocomplete::{build_query, Filters},
        adapters::secondary::elasticsearch,
        domain::ports::remote::Remote,
        domain::ports::search::SearchParameters,
        domain::usecases::search_documents::{SearchDocuments, SearchDocumentsParameters},
        domain::usecases::UseCase,
    };
    use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street, MimirObject};

    pub fn rank(id: &str, list: &[serde_json::Value]) -> Option<usize> {
        list.iter()
            .enumerate()
            // .find(|(_i, v)| v.as_object().unwrap().get("id").unwrap().as_str().unwrap() == id)
            .find(|(_i, v)| {
                let idr = v.as_object().unwrap().get("id").unwrap().as_str().unwrap();
                println!("id: {}", idr);
                idr == id
            })
            .map(|(i, _r)| i)
    }

    pub fn steps() -> Steps<crate::MyWorld> {
        let mut builder: Steps<crate::MyWorld> = Steps::new();

        builder
            .given_regex_async(
                "(.*) have been loaded using (.*) from (.*)",
                t!(|world, matches, _step| {
                    let _t = matches[1].clone();
                    let _u = matches[2].clone();
                    /* TODO load t with u */
                    world
                }),
            )
            .when_regex_async(
                "the user searches for \"(.*)\"",
                t!(|mut world, matches, _step| {
                    let pool = elasticsearch::remote::connection_test_pool().await.unwrap();

                    let client = pool.conn().await.unwrap();

                    let search_documents = SearchDocuments::new(Box::new(client));

                    let filters = Filters::default();

                    let query = build_query(&matches[1], filters, &["fr"], &world.query_settings);

                    let parameters = SearchDocumentsParameters {
                        parameters: SearchParameters {
                            dsl: query,
                            doc_types: vec![
                                String::from(Admin::doc_type()),
                                String::from(Street::doc_type()),
                                String::from(Addr::doc_type()),
                                String::from(Stop::doc_type()),
                                String::from(Poi::doc_type()),
                            ],
                        },
                    };
                    world.search_result = search_documents.execute(parameters).await.unwrap();
                    world
                }),
            )
            .then_regex(
                r"^he finds (.*) in the first (.*) results.$",
                |world, matches, _step| {
                    let limit = matches[2].parse::<usize>().expect("limit");
                    let rank = rank(&matches[1], &world.search_result).unwrap();
                    assert!(rank < limit);
                    world
                },
            );

        builder
    }
}

#[tokio::main]
async fn main() {
    // Do any setup you need to do before running the Cucumber runner.
    // e.g. setup_some_db_thing()?;

    cucumber::Cucumber::<MyWorld>::new()
        // Specifies where our feature files exist
        .features(&["./features"])
        // Adds the implementation of our steps to the runner
        .steps(example_steps::steps())
        // Parses the command line arguments if passed
        .cli()
        // Runs the Cucumber tests and then exists
        .run_and_exit()
        .await
}
