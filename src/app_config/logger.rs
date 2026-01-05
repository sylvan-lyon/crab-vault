use crab_vault::logger::LogLevel;
use serde::{Deserialize, Serialize};

use crate::{app_config::ConfigItem, error::fatal::FatalResult};

pub type LoggerConfig = StaticLoggerConfig;

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields, default)]
pub struct StaticLoggerConfig {
    /// 最低的日志输出等级
    pub level: LogLevel,

    /// 彩色日志
    pub with_ansi: bool,

    /// 调用日志输出的文件
    pub with_file: bool,

    /// 调用日志输出的模块
    pub with_target: bool,

    /// 展示线程信息
    pub with_thread: bool,

    /// 日志文件输出到哪个文件夹下
    pub dump_path: Option<String>,

    /// 日志文件的最低输出等级
    #[serde(default)]
    pub dump_level: LogLevel,
}

impl ConfigItem for StaticLoggerConfig {
    type RuntimeConfig = Self;

    fn into_runtime(self) -> FatalResult<Self::RuntimeConfig> {
        Ok(self)
    }
}

impl Default for StaticLoggerConfig {
    fn default() -> Self {
        Self {
            level: LogLevel::default(),
            dump_path: None,
            dump_level: LogLevel::default(),
            with_ansi: true,
            with_file: true,
            with_target: true,
            with_thread: true,
        }
    }
}
