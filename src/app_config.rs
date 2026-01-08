use clap::error::ErrorKind;
use serde::{Deserialize, Serialize};

use crate::{
    app_config::{
        auth::{AuthConfig, StaticAuthConfig},
        data::{DataConfig, StaticDataConfig},
        logger::{LoggerConfig, StaticLoggerConfig},
        meta::{MetaConfig, StaticMetaConfig},
        server::{ServerConfig, StaticServerConfig},
    },
    cli::run::RunArgs,
    error::fatal::{FatalError, FatalResult, MultiFatalError},
};

pub mod auth;
pub mod data;
pub mod logger;
pub mod meta;
pub mod server;
pub mod util;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
#[derive(Default, Clone)]
pub struct StaticAppConfig {
    pub auth: StaticAuthConfig,
    pub data: StaticDataConfig,
    pub logger: StaticLoggerConfig,
    pub meta: StaticMetaConfig,
    pub server: StaticServerConfig,
}

#[derive(Clone)]
pub struct AppConfig {
    pub auth: AuthConfig,
    pub data: DataConfig,
    pub logger: LoggerConfig,
    pub meta: MetaConfig,
    pub server: ServerConfig,
}

/// [`ConfigItem`] 表示一个配置项，实现了这个 trait 的结构就是一个配置项
///
/// 一个配置项必须能够转化为某一个 `Self::RuntimeConfig`，这些能够直接在 runtime 获取
///
/// 类似于某一个 cache 之类的概念
///
/// 在这个转换过程中，可能会出现不同的、大量的错误，我们使用 [`MultiFatalError`](crate::error::fatal::MultiFatalError) 表示
pub trait ConfigItem
where
    Self: for<'de> Deserialize<'de> + Clone + Sized + Default,
{
    type RuntimeConfig;
    fn into_runtime(self) -> FatalResult<Self::RuntimeConfig>;

    /// 将 [`self`] 转化为 `Self::RuntimeConfig`，并将产生的错误收集到 `errors` 中
    fn error_recorded(self, errors: &mut MultiFatalError) -> Option<Self::RuntimeConfig> {
        self.into_runtime()
            .map_err(|mut e| errors.append(&mut e))
            .ok()
    }
}

impl StaticAppConfig {
    pub fn from_file(config_path: String) -> Self {
        config::Config::builder()
            .add_source(
                config::File::with_name(&config_path)
                    .required(true)
                    .format(config::FileFormat::Toml),
            )
            .build()
            .unwrap_or_else(|_| {
                FatalError::new(
                    ErrorKind::Io,
                    format!("Cannot read configuration file from {config_path}"),
                    None,
                )
                .exit_now()
            })
            .try_deserialize()
            .unwrap_or_else(|_| {
                FatalError::new(
                    ErrorKind::Io,
                    format!("Cannot deserialize configuration from file {config_path}"),
                    None,
                )
                .exit_now()
            })
    }

    pub fn merge_cli(
        mut self,
        RunArgs {
            port,
            data_source,
            meta_source,
            log_level,
            dump_path,
            dump_level,
        }: RunArgs,
    ) -> Self {
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
            self.logger.dump_level = dump_level
        }

        self
    }
}

impl ConfigItem for StaticAppConfig {
    type RuntimeConfig = AppConfig;

    fn into_runtime(self) -> FatalResult<Self::RuntimeConfig> {
        let StaticAppConfig {
            auth,
            data,
            logger,
            meta,
            server,
        } = self;

        let mut errors = MultiFatalError::new();

        let (auth, data, logger, meta, server) = (
            auth.error_recorded(&mut errors),
            data.error_recorded(&mut errors),
            logger.error_recorded(&mut errors),
            meta.error_recorded(&mut errors),
            server.error_recorded(&mut errors),
        );

        if !errors.is_empty() {
            Err(errors)
        } else {
            Ok(AppConfig {
                auth: auth.unwrap(),
                data: data.unwrap(),
                logger: logger.unwrap(),
                meta: meta.unwrap(),
                server: server.unwrap(),
            })
        }
    }
}
