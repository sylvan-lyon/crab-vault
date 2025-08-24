use clap::Parser;

use crate::cli::{Action, Cli};

mod api;
mod app_config;
mod cli;
mod errors;
mod logger;
mod server;
mod storage;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.action() {
        Action::Run => server::run().await,
        Action::Config => cli::run().await,
    }
}
