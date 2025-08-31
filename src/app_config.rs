use std::{collections::HashMap, sync::LazyLock};

use clap::{CommandFactory, Parser, error::ErrorKind};
use serde::{Deserialize, Serialize};

use crate::{
    app_config::{data::DataConfig, logger::LoggerConfig, meta::MetaConfig, server::ServerConfig},
    cli::{Cli, CliCommand, run::RunArgs},
};

pub mod data;
pub mod logger;
pub mod meta;
pub mod server;

static CONFIG: LazyLock<AppConfig> =
    LazyLock::new(|| AppConfig::build_from_config_file().override_by_cli(Cli::parse()));

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

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
#[derive(Default)]
pub struct AppConfig {
    pub(super) server: ServerConfig,
    pub(super) data: DataConfig,
    pub(super) meta: MetaConfig,
    pub(super) logger: LoggerConfig,
}

impl AppConfig {
    pub fn get_field_value_map() -> HashMap<&'static str, toml_edit::Item> {
        use toml_edit::{Item, Value};
        HashMap::from([
            ("server.port", Item::Value(Value::from(0))),
            ("data.source", Item::Value(Value::from(""))),
            ("meta.source", Item::Value(Value::from(""))),
            ("logger.level", Item::Value(Value::from(""))),
            ("logger.dump_path", Item::Value(Value::from(""))),
            ("logger.with_ansi", Item::Value(Value::from(true))),
            ("logger.with_file", Item::Value(Value::from(true))),
            ("logger.with_target", Item::Value(Value::from(true))),
            ("logger.with_thread", Item::Value(Value::from(true))),
        ])
    }

    pub fn get_valid_paths() -> HashMap<&'static str, toml_edit::Item> {
        use toml_edit::{Item, Table, Value};
        HashMap::from([
            ("server", Item::Table(Table::new())),
            ("data", Item::Table(Table::new())),
            ("meta", Item::Table(Table::new())),
            ("logger", Item::Table(Table::new())),
            ("server.port", Item::Value(Value::from(0))),
            ("data.source", Item::Value(Value::from(""))),
            ("meta.source", Item::Value(Value::from(""))),
            ("logger.level", Item::Value(Value::from(""))),
            ("logger.dump_path", Item::Value(Value::from(""))),
            ("logger.with_ansi", Item::Value(Value::from(true))),
            ("logger.with_file", Item::Value(Value::from(true))),
            ("logger.with_target", Item::Value(Value::from(true))),
            ("logger.with_thread", Item::Value(Value::from(true))),
        ])
    }

    pub fn build_from_config_file() -> Self {
        let Cli {
            subcommand: _,
            config_path,
        } = Cli::parse();

        config::Config::builder()
            .add_source(
                config::File::with_name(&config_path)
                    .required(false)
                    .format(config::FileFormat::Toml),
            )
            .build()
            .unwrap_or_else(|e| {
                Cli::command()
                    .error(
                        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
                        format!("Cannot deserialize the configuration file, details:\n\n    {e}"),
                    )
                    .exit();
            })
            .try_deserialize()
            .unwrap_or_else(|e| {
                Cli::command()
                    .error(
                        ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
                        format!("Cannot understand the configuration file, details:\n\n    {e}"),
                    )
                    .exit();
            })
    }

    pub fn override_by_cli(mut self, cli: Cli) -> Self {
        let Cli {
            subcommand: cli,
            config_path: _,
        } = cli;
        match cli {
            CliCommand::Run(run_args) => {
                let RunArgs {
                    port,
                    data_source,
                    meta_source,
                    log_level,
                    dump_path,
                    dump_level,
                } = run_args;

                if let Some(port) = port {
                    self.server.port = port
                }

                if let Some(data_source) = data_source {
                    self.data.source = data_source
                }

                if let Some(meta_source) = meta_source {
                    self.meta.source = meta_source
                }

                if let Some(log_level) = log_level {
                    self.logger.level = log_level
                }

                if let Some(dump_path) = dump_path {
                    self.logger.dump_path = Some(dump_path)
                }

                if let Some(dump_level) = dump_level {
                    self.logger.dump_level = Some(dump_level)
                }
            }
            CliCommand::Jwt(_) => {}
            CliCommand::Config(_) => {}
        };
        self
    }
}
