use serde::Serialize;

use mimir2::adapters::secondary::elasticsearch;
use mimir2::adapters::secondary::elasticsearch::internal::{
    IndexConfiguration, IndexMappings, IndexParameters, IndexSettings,
};
use mimir2::domain::model::configuration::Configuration;
use mimir2::domain::model::document::Document;
use mimir2::domain::ports::remote::Remote;
use mimir2::domain::ports::storage::Storage;

#[derive(Serialize)]
struct TestObj {
    value: String,
}

impl Document for TestObj {
    const IS_GEO_DATA: bool = false;
    const DOC_TYPE: &'static str = "obj";

    fn id(&self) -> String {
        self.value.clone()
    }
}

#[tokio::main]
async fn main() {
    let pool = elasticsearch::remote::connection_pool("http://localhost:9200")
        .await
        .expect("connection pool");
    let client = pool.conn().await.expect("client connection");
    let config = IndexConfiguration {
        name: String::from("test-index"),
        parameters: IndexParameters {
            timeout: String::from("10s"),
            wait_for_active_shards: String::from("1"), // only the primary shard
        },
        settings: IndexSettings {
            value: String::from(include_str!("../../../config/admin/settings.json")), // <<=== Invalid Settings
        },
        mappings: IndexMappings {
            value: String::from(include_str!("../../../config/admin/mappings.json")),
        },
    };
    let root_config = Configuration {
        value: serde_json::to_string(&config).expect("config"),
    };
    let config = root_config
        .clone()
        .normalize_index_name(TestObj::DOC_TYPE)
        .expect("normalize index name");
    println!("config: {:?}", config);
    let res = client.create_container(config.clone()).await.unwrap();
    println!("res: {:?}", res);
    // let alias = configuration::root_doctype_dataset(TestObj::DOC_TYPE, "test-index");
    // let _ = client.create_alias(res.name.clone(), alias).await.unwrap();
    // let alias = configuration::root_doctype(TestObj::DOC_TYPE);
    // let _ = client.create_alias(res.name.clone(), alias).await.unwrap();
    // let alias = configuration::root();
    // let _ = client.create_alias(res.name.clone(), alias).await.unwrap();
    // let _ = client.find_aliases(res.name.clone()).await.unwrap();
    // let config = root_config
    //     .clone()
    //     .normalize_index_name(TestObj::DOC_TYPE)
    //     .expect("normalize index name");
    // let res = client.create_container(config).await.unwrap();
    // println!("created index: {:?}", res);
    // let alias = configuration::root_doctype_dataset(TestObj::DOC_TYPE, "test-index");
    // let _ = client.create_alias(res.name.clone(), alias).await.unwrap();
    // let alias = configuration::root_doctype(TestObj::DOC_TYPE);
    // let _ = client.create_alias(res.name.clone(), alias).await.unwrap();
    // let alias = configuration::root();
    // let _ = client.create_alias(res.name.clone(), alias).await.unwrap();
    // let base_index = configuration::root_doctype_dataset(TestObj::DOC_TYPE, "test-index");
    // let aliases = client.find_aliases(base_index).await.unwrap();
    // println!("aliases: {:?}", aliases);
}
