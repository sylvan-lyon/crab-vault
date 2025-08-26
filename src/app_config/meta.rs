use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct MetaConfig {
    pub(super) source: String,
}

impl Default for MetaConfig {
    fn default() -> Self {
        Self {
            source: "./meta".to_string(),
        }
    }
}

impl MetaConfig {
    pub fn source(&self) -> &str {
        &self.source
    }
}
