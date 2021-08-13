use cucumber::{async_trait, criteria::feature, futures::FutureExt, Context, Cucumber, World};
use elasticsearch::http::transport::SingleNodeConnectionPool;
use mimir2::adapters::secondary::elasticsearch::remote::connection_test_pool;
use mimir2::domain::ports::remote::Remote;
use std::convert::Infallible;

mod steps;

pub enum MyWorld {
    Nothing,
    SomeString(String),
    SuffixedString(String),
    TwoStrings(String, String),
}

#[async_trait(?Send)]
impl World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self::Nothing)
    }
}

#[tokio::main]
async fn main() {
    let pool = connection_test_pool().await.unwrap();

    Cucumber::<MyWorld>::new()
        // Specifies where our feature files exist
        .features(&["./features/admin"])
        // Adds the implementation of our steps to the runner
        .steps(steps::example::steps())
        // Add some global context for all the tests, like databases.
        .context(Context::new().add(pool))
        // Add some lifecycle functions to manage our database nightmare
        .before(feature("Example feature"), |ctx| {
            let pool = ctx.get::<SingleNodeConnectionPool>().unwrap().clone();
            async move {
                let _client = pool.conn().await.unwrap();
            }
            .boxed()
        })
        // .after(feature("Example feature"), |ctx| {
        //     let pool = ctx.get::<SqlitePool>().unwrap().clone();
        //     async move { drop_tables(&pool).await }.boxed()
        // })
        // Parses the command line arguments if passed
        .cli()
        // Runs the Cucumber tests and then exists
        .run_and_exit()
        .await
}

// use cucumber::async_trait;
// use mimir2::adapters::primary::bragi::settings::QuerySettings;
// use snafu::{ResultExt, Snafu};
// use std::convert::Infallible;
// use std::env;
// use std::path::PathBuf;
// use tokio::io::AsyncWriteExt;
// use url::Url;
//
// pub struct MyWorld {
//     query_settings: QuerySettings,
//     search_result: Vec<serde_json::Value>,
// }
//
// #[async_trait(?Send)]
// impl cucumber::World for MyWorld {
//     type Error = Infallible;
//
//     async fn new() -> Result<Self, Infallible> {
//         let mut query_settings_file = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
//         query_settings_file.push("config/query_settings.toml");
//         let query_settings = QuerySettings::new_from_file(query_settings_file)
//             .await
//             .expect("query settings");
//         Ok(Self {
//             query_settings,
//             search_result: Vec::new(),
//         })
//     }
// }
//
// mod example_steps {
//     use cucumber::{t, Steps};
//     use log::*;
//     // use failure::format_err;
//     use super::download_osm;
//     use mimir2::{
//         adapters::primary::bragi::autocomplete::{build_query, Filters},
//         adapters::secondary::elasticsearch,
//         domain::ports::remote::Remote,
//         domain::ports::search::SearchParameters,
//         domain::usecases::search_documents::{SearchDocuments, SearchDocumentsParameters},
//         domain::usecases::UseCase,
//     };
//     use places::{addr::Addr, admin::Admin, poi::Poi, stop::Stop, street::Street, MimirObject};
//
//     pub fn rank(id: &str, list: &[serde_json::Value]) -> Option<usize> {
//         list.iter()
//             .enumerate()
//             // .find(|(_i, v)| v.as_object().unwrap().get("id").unwrap().as_str().unwrap() == id)
//             .find(|(_i, v)| {
//                 let idr = v.as_object().unwrap().get("id").unwrap().as_str().unwrap();
//                 info!("id: {}", idr);
//                 idr == id
//             })
//             .map(|(i, _r)| i)
//     }
//
//     pub fn steps() -> Steps<crate::MyWorld> {
//         let mut builder: Steps<crate::MyWorld> = Steps::new();
//
//         builder
//             .given_regex_async(
//                 "(.*) have been loaded using (.*) from (.*)",
//                 t!(|world, matches, _step| {
//                     let _t = matches[1].clone();
//                     let u = matches[2].clone();
//                     download_osm(&u).await.unwrap();
//                     world
//                 }),
//             )
//             .when_regex_async(
//                 "the user searches for \"(.*)\"",
//                 t!(|mut world, matches, _step| {
//                     let pool = elasticsearch::remote::connection_test_pool().await.unwrap();
//
//                     let client = pool.conn().await.unwrap();
//
//                     let search_documents = SearchDocuments::new(Box::new(client));
//
//                     let filters = Filters::default();
//
//                     let query = build_query(&matches[1], filters, &["fr"], &world.query_settings);
//
//                     info!("She pretty");
//
//                     let parameters = SearchDocumentsParameters {
//                         parameters: SearchParameters {
//                             dsl: query,
//                             doc_types: vec![
//                                 String::from(Admin::doc_type()),
//                                 String::from(Street::doc_type()),
//                                 String::from(Addr::doc_type()),
//                                 String::from(Stop::doc_type()),
//                                 String::from(Poi::doc_type()),
//                             ],
//                         },
//                     };
//                     world.search_result = search_documents.execute(parameters).await.unwrap();
//                     world
//                 }),
//             )
//             .then_regex(
//                 r"^he finds (.*) in the first (.*) results.$",
//                 |world, matches, _step| {
//                     let limit = matches[2].parse::<usize>().expect("limit");
//                     let rank = rank(&matches[1], &world.search_result).unwrap();
//                     assert!(rank < limit);
//                     world
//                 },
//             );
//
//         builder
//     }
// }
//
// #[tokio::main]
// async fn main() {
//     // Do any setup you need to do before running the Cucumber runner.
//     // e.g. setup_some_db_thing()?;
//
//     let _ = env_logger::builder().is_test(true).try_init();
//     cucumber::Cucumber::<MyWorld>::new()
//         // Specifies where our feature files exist
//         .features(&["./features/admin"])
//         // Adds the implementation of our steps to the runner
//         .steps(example_steps::steps())
//         // Parses the command line arguments if passed
//         .cli()
//         // Runs the Cucumber tests and then exists
//         .run_and_exit()
//         .await
// }
//
