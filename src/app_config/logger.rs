use clap::ValueEnum;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct LoggerConfig {
    /// 最低的日志输出等级
    pub(super) level: LogLevel,

    /// 彩色日志
    pub(super) with_ansi: bool,

    /// 调用日志输出的文件
    pub(super) with_file: bool,

    /// 调用日志输出的模块
    pub(super) with_target: bool,

    /// 展示线程信息
    pub(super) with_thread: bool,

    /// 日志文件输出到哪个文件夹下
    pub(super) dump_path: Option<String>,

    /// 日志文件的最低输出等级
    pub(super) dump_level: Option<LogLevel>,
}

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
    /// ### 这也意味着如果 `dump_path.is_some()` 成立，这个函数的返回值就可以直接 `unwrap()`，如果未指定，将返回 Warn
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
