use mimir2::adapters::secondary::elasticsearch::remote;

#[tokio::main]
async fn main() {
    let pool = remote::connection_pool_url("foobar").await.unwrap();
}
