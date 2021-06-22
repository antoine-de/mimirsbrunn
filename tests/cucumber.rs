use cucumber::async_trait;
use std::{cell::RefCell, convert::Infallible};

pub struct MyWorld {
    // You can use this struct for mutable context in scenarios.
    foo: String,
    bar: usize,
    some_value: RefCell<u8>,
}

impl MyWorld {
    async fn test_async_fn(&mut self) {
        *self.some_value.borrow_mut() = 123u8;
        self.bar = 123;
    }
}

#[async_trait(?Send)]
impl cucumber::World for MyWorld {
    type Error = Infallible;

    async fn new() -> Result<Self, Infallible> {
        Ok(Self {
            foo: "wat".into(),
            bar: 0,
            some_value: RefCell::new(0),
        })
    }
}

mod example_steps {
    use cucumber::{t, Steps};
    // use failure::format_err;
    use futures::stream::StreamExt;
    use lazy_static::lazy_static;
    use mimir::objects::Admin;
    use mimir2::{
        adapters::secondary::elasticsearch::{
            self,
            internal::{IndexConfiguration, IndexMappings, IndexParameters, IndexSettings},
        },
        domain::model::query_parameters::QueryParameters,
        domain::ports::remote::Remote,
        domain::usecases::search_documents::{SearchDocuments, SearchDocumentsParameters},
        domain::usecases::UseCase,
    };
    use mimirsbrunn::bano::Bano;
    use slog_scope::info;
    use std::path::PathBuf;
    use std::sync::Arc;
    use structopt::StructOpt;

    pub fn steps() -> Steps<crate::MyWorld> {
        let mut builder: Steps<crate::MyWorld> = Steps::new();

        builder
            .given_regex_async(
                "(.*) have been loaded using (.*) from (.*)",
                t!(|world, matches, _step| {
                    let t = matches[1].clone();
                    let u = matches[2].clone();
                    //let v = matches[3].clone();
                    println!("TODO: load {} with {}", t, u);
                    // world.foo = "elho".into();
                    // world.test_async_fn().await;
                    world
                }),
            )
            .when_regex_async(
                "the user searches for \"(.*)\"",
                t!(|world, matches, _step| {
                    let pool = elasticsearch::remote::connection_test_pool().await.unwrap();

                    let client = pool.conn().await.unwrap();

                    let search_documents = SearchDocuments::new(Box::new(client));
                    let parameters = SearchDocumentsParameters {
                        query_parameters: QueryParameters {
                            containers: vec![String::from("munin")],
                            dsl: String::from(r#"{ "match_all": {} }"#),
                        },
                    };
                    let stream = search_documents.execute(parameters).await.unwrap();

                    let admins = admin_stream
                        .map(|v| serde_json::from_value(v).expect("cannot deserialize admin"))
                        .collect::<Vec<Admin>>()
                        .await;

                    let t = matches[1].clone();
                    println!("Search query {}", t);
                    world
                }),
            )
            .then_regex(
                r"^he finds (.*) in the first (.*) results.$",
                |world, matches, _step| {
                    // And access them as an array
                    assert_eq!(matches[1], "implement");
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
