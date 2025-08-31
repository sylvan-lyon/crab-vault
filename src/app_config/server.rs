use std::{
    collections::HashSet,
    hash::Hash,
};

use crab_vault_auth::HttpMethod;
use glob::Pattern;
use serde::{Deserialize, Serialize};

use crate::http::auth::JwtConfigBuilder;

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ServerConfig {
    pub(super) port: u16,
    pub(super) auth: AuthConfig,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct AuthConfig {
    /// 这里使用 Vec
    ///
    /// 在编译规则时保证如果同一个路径下有多种公开方式时，采取最后指定的公开请求方法而非并集
    #[serde(default)]
    pub(super) path_rules: Vec<PathRule>,

    /// jwt 鉴权相关设置
    #[serde(default)]
    pub(super) jwt_config: JwtConfigBuilder,
}

#[derive(Default, Serialize, Deserialize, Clone)]
pub struct PathRule {
    /// 路径的通配符，UNIX shell 通配符
    pub(super) pattern: String,

    /// 无需 token 即可访问的那些方法
    #[serde(default)]
    pub(super) public_methods: HashSet<HttpMethod>,
}

impl ServerConfig {
    pub fn port(&self) -> u16 {
        self.port
    }

    pub fn auth(&self) -> &AuthConfig {
        &self.auth
    }
}

impl AuthConfig {
    pub fn jwt_config_builder(&self) -> &JwtConfigBuilder {
        &self.jwt_config
    }

    pub fn get_compiled_path_rules(&self) -> Vec<(Pattern, HashSet<HttpMethod>)> {
        self.path_rules
            .iter()
            .cloned()
            .filter_map(|rule| rule.compile())
            .collect()
    }
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            port: 32767,
            auth: AuthConfig::default(),
        }
    }
}

impl PartialEq for PathRule {
    fn eq(&self, other: &Self) -> bool {
        self.pattern == other.pattern
    }
}

impl Eq for PathRule {}

impl Hash for PathRule {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.pattern.hash(state);
    }
}

impl PathRule {
    pub fn compile(&self) -> Option<(Pattern, HashSet<HttpMethod>)> {
        match Pattern::new(&self.pattern) {
            Ok(val) => Some((val, self.public_methods.iter().copied().collect())),
            Err(e) => {
                tracing::error!(
                    "the PATH `{}` of path rules is not written in valid UNIX shell format, so this pattern is skipped, if that matters, please check your configuration file, details: {e}",
                    self.pattern
                );
                None
            }
        }
    }
}
