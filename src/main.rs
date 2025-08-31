mod app_config;
mod cli;
mod error;
mod http;
mod logger;

#[tokio::main]
async fn main() {
    cli::run().await
}
