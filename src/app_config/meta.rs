use serde::{Deserialize, Serialize};

use crate::{app_config::ConfigItem, error::fatal::FatalResult};

pub type MetaConfig = StaticMetaConfig;

#[derive(Deserialize, Serialize, Clone)]
#[serde(deny_unknown_fields, default)]
pub struct StaticMetaConfig {
    pub source: String,
}

impl Default for StaticMetaConfig {
    fn default() -> Self {
        Self {
            source: std::env::home_dir()
                .map(|v| {
                    v.join(".local/state/crab-vault/data")
                        .to_string_lossy()
                        .into()
                })
                .unwrap_or("./data".into()),
        }
    }
}

impl ConfigItem for StaticMetaConfig {
    type RuntimeConfig = Self;

    fn into_runtime(self) -> FatalResult<Self::RuntimeConfig> {
        Ok(self)
    }
}
