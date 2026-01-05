use serde::{Deserialize, Serialize};

use crate::{app_config::ConfigItem, error::fatal::FatalResult};

pub type ServerConfig = StaticServerConfig;

#[derive(Deserialize, Serialize, Default, Clone)]
#[serde(deny_unknown_fields, default)]
pub struct StaticServerConfig {
    #[serde(default = "ServerConfig::default_port")]
    pub port: u16,
}


impl StaticServerConfig {
    const fn default_port() -> u16 {
        32767
    }
}

impl ConfigItem for StaticServerConfig {
    type RuntimeConfig = Self;

    fn into_runtime(self) -> FatalResult<Self::RuntimeConfig> {
        Ok(self)
    }
}
