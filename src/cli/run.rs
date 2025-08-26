use clap::Args;

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
    pub log_level: Option<String>,

    /// Log file dump path, or no log file will be saved
    #[arg(long = "dump-path", short = None)]
    pub dump_path: Option<String>,
}
