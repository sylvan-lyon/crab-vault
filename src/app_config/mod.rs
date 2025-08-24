use std::sync::LazyLock;

use clap::Parser;

use crate::{
    app_config::config::{
        data::DataConfig, logger::LoggerConfig, meta::MetaConfig, server::ServerConfig, AppConfig
    },
    cli::Cli,
};

pub mod config;

static CONFIG: LazyLock<config::AppConfig> = LazyLock::new(|| {
    let conf = AppConfig::build_from_config_file().override_by_cli(Cli::parse());
    conf
});

pub fn server() -> &'static ServerConfig {
    &CONFIG.server
}

pub fn data() -> &'static DataConfig {
    &CONFIG.data
}

pub fn meta() -> &'static MetaConfig {
    &CONFIG.meta
}

pub fn logger() -> &'static LoggerConfig {
    &CONFIG.logger
}
