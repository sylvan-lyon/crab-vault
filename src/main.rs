use clap::Parser;

use crate::{cli::{Action, Cli}, http::server};

mod app_config;
mod cli;
mod error;
mod http;
mod logger;
mod storage;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();
    match cli.action() {
        Action::Run => server::run().await,
        Action::Config => cli::run().await,
    }
}
