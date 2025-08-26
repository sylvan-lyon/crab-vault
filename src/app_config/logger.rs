use std::str::FromStr;

use clap::error::ErrorKind;
use serde::{Deserialize, Serialize};

use crate::error::cli::CliError;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct LoggerConfig {
    pub(super) level: LogLevel,
    pub(super) with_ansi: bool,
    pub(super) with_file: bool,
    pub(super) with_target: bool,
    pub(super) with_thread: bool,
    pub(super) dump_path: Option<String>,
    pub(super) dump_level: Option<LogLevel>,
}

#[derive(Deserialize, Serialize, PartialEq, PartialOrd, Clone, Copy, Debug, Default)]
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

impl Default for LoggerConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::Trace,
            dump_path: None,
            dump_level: None,
            with_ansi: true,
            with_file: true,
            with_target: true,
            with_thread: true,
        }
    }
}

impl LoggerConfig {
    pub fn level(&self) -> LogLevel {
        self.level
    }

    pub fn dump_path(&self) -> Option<&str> {
        match &self.dump_path {
            Some(val) => Some(val),
            None => None,
        }
    }

    /// dump_level 完全依赖于 `dump_path` ，只有在设置 `dump_path` 之后，才会有 `dump_path` ，否则此值无意义
    ///
    /// ### 这也意味着如果 `dump_path.is_some()` 成立，这个函数的返回值就可以直接 `unwrap()`
    pub fn dump_level(&self) -> Option<LogLevel> {
        if self.dump_path().is_some() {
            match self.dump_level {
                Some(val) => Some(val),
                None => Some(LogLevel::Warn),
            }
        } else {
            None
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

impl FromStr for LogLevel {
    type Err = CliError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "trace" => Ok(LogLevel::Trace),
            "debug" => Ok(LogLevel::Debug),
            "info" => Ok(LogLevel::Info),
            "warn" => Ok(LogLevel::Warn),
            "error" => Ok(LogLevel::Error),
            _ => Err(CliError::new(
                ErrorKind::InvalidValue,
                "the value cannot be parsed as log level".to_string(),
                None,
            )),
        }
    }
}
