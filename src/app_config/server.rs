use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct ServerConfig {
    #[serde(default = "ServerConfig::default_port")]
    pub(super) port: u16,
}


impl ServerConfig {
    const fn default_port() -> u16 {
        32767
    }
    
    pub fn port(&self) -> u16 {
        self.port
    }
}
