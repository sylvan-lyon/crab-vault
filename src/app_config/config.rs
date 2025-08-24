use std::collections::HashMap;

use clap::{CommandFactory, Parser, error::ErrorKind};
use serde::{Deserialize, Serialize};

use crate::{
    app_config::config::{
        data::DataConfig, logger::LoggerConfig, meta::MetaConfig, server::ServerConfig,
    },
    cli::{Cli, CliCommand},
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
#[derive(Default)]
pub struct AppConfig {
    pub(super) server: ServerConfig,
    pub(super) data: DataConfig,
    pub(super) meta: MetaConfig,
    pub(super) logger: LoggerConfig,
}

pub mod server {
    use super::*;
    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields, default)]
    pub struct ServerConfig {
        pub(super) port: u16,
    }

    impl Default for ServerConfig {
        fn default() -> Self {
            Self { port: 32767 }
        }
    }

    impl ServerConfig {
        pub fn port(&self) -> u16 {
            self.port
        }
    }
}

pub mod data {
    use super::*;

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields, default)]
    pub struct DataConfig {
        pub(super) source: String,
    }

    impl Default for DataConfig {
        fn default() -> Self {
            Self {
                source: "./data".to_string(),
            }
        }
    }

    impl DataConfig {
        pub fn source(&self) -> &str {
            &self.source
        }
    }
}

pub mod meta {
    use super::*;

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields, default)]
    pub struct MetaConfig {
        pub(super) source: String,
    }

    impl Default for MetaConfig {
        fn default() -> Self {
            Self {
                source: "./meta".to_string(),
            }
        }
    }

    impl MetaConfig {
        pub fn source(&self) -> &str {
            &self.source
        }
    }
}

pub mod logger {
    use super::*;

    #[derive(Deserialize, Serialize)]
    #[serde(deny_unknown_fields, default)]
    pub struct LoggerConfig {
        pub(super) level: String,
        pub(super) with_ansi: bool,
        pub(super) with_file: bool,
        pub(super) with_target: bool,
        pub(super) with_thread: bool,
        pub(super) dump_path: Option<String>,
    }

    impl Default for LoggerConfig {
        fn default() -> Self {
            Self {
                level: "trace".to_string(),
                dump_path: None,
                with_ansi: true,
                with_file: true,
                with_target: true,
                with_thread: true,
            }
        }
    }

    impl LoggerConfig {
        pub fn level(&self) -> &str {
            &self.level
        }

        pub fn dump_path(&self) -> Option<&str> {
            match &self.dump_path {
                Some(val) => Some(val),
                None => None,
            }
        }

        pub fn with_ansi(&self) -> bool {
            self.with_ansi
        }

        pub fn with_file(&self) -> bool {
            self.with_file
        }

        pub fn with_target(&self) -> bool {
            self.with_target
        }

        pub fn with_thread(&self) -> bool {
            self.with_thread
        }
    }
}

impl AppConfig {
    pub fn get_field_value_map() -> HashMap<&'static str, toml_edit::Item> {
        use toml_edit::{Item, Value};
        HashMap::from([
            ("server.port", Item::Value(/* i16 */ Value::from(0))),
            ("data.source", Item::Value(/* String */ Value::from(""))),
            ("meta.source", Item::Value(/* String */ Value::from(""))),
            ("logger.level", Item::Value(/* String */ Value::from(""))),
            ("logger.dump_path", Item::Value(/* Option<String> */ Value::from(""))),
            ("logger.with_ansi", Item::Value(/* bool */ Value::from(true))),
            ("logger.with_file", Item::Value(/* bool */ Value::from(true))),
            ("logger.with_target", Item::Value(/* bool */ Value::from(true))),
            ("logger.with_thread", Item::Value(/* bool */ Value::from(true))),
        ])
    }

    pub fn get_valid_paths() -> HashMap<&'static str, toml_edit::Item> {
        use toml_edit::{Item, Value, Table};
        HashMap::from([
            ("server", Item::Table(Table::new())),
            ("data", Item::Table(Table::new())),
            ("meta", Item::Table(Table::new())),
            ("logger", Item::Table(Table::new())),

            ("server.port", Item::Value(/* i16 */ Value::from(0))),
            ("data.source", Item::Value(/* String */ Value::from(""))),
            ("meta.source", Item::Value(/* String */ Value::from(""))),
            ("logger.level", Item::Value(/* String */ Value::from(""))),
            ("logger.dump_path", Item::Value(/* Option<String> */ Value::from(""))),
            ("logger.with_ansi", Item::Value(/* bool */ Value::from(true))),
            ("logger.with_file", Item::Value(/* bool */ Value::from(true))),
            ("logger.with_target", Item::Value(/* bool */ Value::from(true))),
            ("logger.with_thread", Item::Value(/* bool */ Value::from(true))),
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
            CliCommand::Run {
                port,
                data_source,
                meta_source,
                log_level,
                dump_path,
            } => {
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
            }
            CliCommand::Config(_) => unreachable!(),
        };
        self
    }
}
