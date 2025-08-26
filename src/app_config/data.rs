use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct DataConfig {
    pub(super) source: String,
}

impl Default for DataConfig {
    fn default() -> Self {
        Self {
            source: "./data".to_string(),
        }
    }
}

impl DataConfig {
    pub fn source(&self) -> &str {
        &self.source
    }
}
