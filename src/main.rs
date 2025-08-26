mod app_config;
mod cli;
mod error;
mod http;
mod logger;
mod storage;

#[tokio::main]
async fn main() {
    cli::run().await
}
