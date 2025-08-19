use std::sync::LazyLock;

use clap::{CommandFactory, Parser};
use serde::Deserialize;

pub static CONFIG: LazyLock<AppConfig> = LazyLock::new(|| {
    use clap::error::ErrorKind;
    let default_conf = AppConfig::default();
    let cli_conf = AppConfig::parse();
    let file_conf: AppConfig = config::Config::builder()
        .add_source(
            config::File::with_name("crab-vault.toml")
                .required(false)
                .format(config::FileFormat::Toml),
        )
        .build()
        .unwrap_or_else(|e| {
            AppConfig::command()
                .error(
                    ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
                    format!("Fail to read the configuration file, details: {e}"),
                )
                .exit();
        })
        .try_deserialize()
        .unwrap_or_else(|e| {
            AppConfig::command()
                .error(
                    ErrorKind::DisplayHelpOnMissingArgumentOrSubcommand,
                    format!("Cannot understand the configuration file, details:\n{e}"),
                )
                .exit();
        });

    let curr_conf = default_conf;
    let curr_conf = file_conf.overwrite(curr_conf);
    let curr_conf = cli_conf.overwrite(curr_conf);

    if curr_conf.data_source() == curr_conf.meta_source() {
        AppConfig::command()
            .error(
                ErrorKind::ArgumentConflict,
                "The data source is identical to meta source WRONGLY.",
            )
            .exit();
    }

    curr_conf
});

mod default {
    pub(super) const PORT: u16 = 32767;
    pub(super) const DATA_MNT_POINT: &str = "./data";
    pub(super) const META_MNT_POINT: &str = "./meta";
    pub(super) const LOG_LEVEL: &str = "info";
}

#[derive(Parser, Deserialize)]
#[command(version, author, about, long_about = None)]
#[serde(deny_unknown_fields)]
pub struct AppConfig {
    /// Sets the listening port number of server.
    #[arg(long = "port", short = 'p')]
    port: Option<u16>,

    /// Sepcify the mount point of `data`
    #[arg(long = "data-source", short = 'D')]
    data_source: Option<String>,

    /// Specify the mount point of `meta`
    #[arg(long = "meta-source", short = 'M')]
    meta_source: Option<String>,

    /// Set the minimum log level of server.
    #[arg(long = "log-level", short = 'L')]
    log_level: Option<String>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            port: Some(default::PORT),
            data_source: Some(default::DATA_MNT_POINT.to_string()),
            meta_source: Some(default::META_MNT_POINT.to_string()),
            log_level: Some(default::LOG_LEVEL.to_string()),
        }
    }
}

impl AppConfig {
    fn overwrite(self, rhs: Self) -> Self {
        Self {
            port: self.port.or(rhs.port),
            data_source: self.data_source.or(rhs.data_source),
            meta_source: self.meta_source.or(rhs.meta_source),
            log_level: self.log_level.or(rhs.log_level),
        }
    }

    pub fn port(&self) -> u16 {
        self.port.unwrap_or(32767)
    }

    pub fn data_source(&self) -> &str {
        match &self.data_source {
            Some(val) => val,
            None => "./data",
        }
    }

    pub fn meta_source(&self) -> &str {
        match &self.meta_source {
            Some(val) => val,
            None => "./meta",
        }
    }

    pub fn log_level(&self) -> &str {
        match &self.log_level {
            Some(val) => val,
            None => "info",
        }
    }
}
