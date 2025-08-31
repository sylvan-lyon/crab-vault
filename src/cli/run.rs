use clap::Args;

use crate::app_config::logger::LogLevel;

#[derive(Args)]
pub struct RunArgs {
    /// Listening port number of server.
    #[arg(long = "port", short = 'p')]
    pub port: Option<u16>,

    /// Specify the source of `data`.
    #[arg(long = "data-source", short = None)]
    pub data_source: Option<String>,

    /// Specify the source of `meta`.
    #[arg(long = "meta-source", short = None)]
    pub meta_source: Option<String>,

    /// Minimum log level of server.
    #[arg(long = "log-level", short = 'L')]
    pub log_level: Option<LogLevel>,

    /// Log file dump path, or no log file will be saved
    #[arg(long = "dump-path", short = None)]
    pub dump_path: Option<String>,

    /// The minimum level of dumped logs, default to the configuration file or `WARN`
    #[arg(long = "dump-level", short = None)]
    pub dump_level: Option<LogLevel>,
}
