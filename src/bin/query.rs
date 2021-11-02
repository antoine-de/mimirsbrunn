use common::document::ContainerDocument;
use mimir2::{
    adapters::primary::common::{
        coord::Coord, dsl::build_query, filters::Filters, settings::QuerySettings,
    },
    adapters::secondary::elasticsearch::remote::connection_test_pool,
    domain::model::query::Query,
    domain::ports::primary::search_documents::SearchDocuments,
    domain::ports::secondary::remote::Remote,
};
use places::addr::Addr;
use places::admin::Admin;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
#[structopt(name = "query", about = "Querying Bragi from the commandline")]
struct Opt {
    /// Activate debug mode
    // short and long flags (-d, --debug) will be deduced from the field's name
    #[structopt(short, long)]
    debug: bool,

    /// latitude
    #[structopt(long = "lat")]
    latitude: Option<f32>,

    /// longitude
    #[structopt(long = "lon")]
    longitude: Option<f32>,

    /// Search String
    q: String,
}

#[tokio::main]
async fn main() {
    let opt = Opt::from_args();

    let client = connection_test_pool()
        .conn(Default::default())
        .await
        .expect("Elasticsearch Connection Established");

    let filters = match opt.latitude {
        Some(latitude) => {
            let longitude = opt.longitude.expect("longitude");
            Filters {
                coord: Some(Coord::new(longitude, latitude)),
                ..Default::default()
            }
        }
        None => Filters::default(),
    };

    let settings = QuerySettings::default();

    let dsl = build_query(&opt.q, filters, &["fr"], &settings);

    println!("{}", dsl);

    client
        .search_documents(
            vec![
                Admin::static_doc_type().to_string(),
                Addr::static_doc_type().to_string(),
            ],
            Query::QueryDSL(dsl),
            None,
        )
        .await
        .unwrap()
        .iter()
        .enumerate()
        .for_each(|(i, v): (_, &serde_json::Value)| {
            println!("{}: {} | {} | {}", i, v["id"], v["name"], v["label"]);
        });
}
