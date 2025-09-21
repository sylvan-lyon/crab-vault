use clap::ValueEnum;
use serde::{Deserialize, Serialize};

pub mod json;
pub mod pretty;

#[derive(Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Debug, Default, ValueEnum)]
pub enum LogLevel {
    #[default]
    #[serde(alias = "trace", alias = "TRACE")]
    Trace,
    #[serde(alias = "debug", alias = "DEBUG")]
    Debug,
    #[serde(alias = "info", alias = "INFO")]
    Info,
    #[serde(alias = "warn", alias = "WARN")]
    Warn,
    #[serde(alias = "error", alias = "ERROR")]
    Error,
}

impl From<tracing::Level> for LogLevel {
    #[inline(always)]
    fn from(value: tracing::Level) -> Self {
        match value {
            tracing::Level::TRACE => Self::Trace,
            tracing::Level::DEBUG => Self::Debug,
            tracing::Level::INFO => Self::Info,
            tracing::Level::WARN => Self::Warn,
            tracing::Level::ERROR => Self::Error,
        }
    }
}
