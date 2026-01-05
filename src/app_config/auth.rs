use std::collections::HashSet;

use crab_vault::auth::HttpMethod;
use glob::Pattern;
use serde::{Deserialize, Serialize};

use crate::{
    app_config::{
        ConfigItem,
        util::{
            JwtDecoderConfig, JwtEncoderConfig, StaticJwtDecoderConfig, StaticJwtEncoderConfig,
        },
    },
    error::fatal::{FatalError, FatalResult, MultiFatalError},
};

#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(deny_unknown_fields, default)]
pub struct StaticAuthConfig {
    /// 这里使用 Vec
    ///
    /// 在编译规则时保证如果同一个路径下有多种公开方式时，采取最后指定的公开请求方法而非并集
    #[serde(default = "StaticAuthConfig::default_path_rules")]
    pub path_rules: Vec<StaticPathRule>,

    #[serde(default)]
    pub jwt_encoder_config: StaticJwtEncoderConfig,

    /// jwt 鉴权相关设置
    #[serde(default)]
    pub jwt_decoder_config: StaticJwtDecoderConfig,
}

#[derive(Clone)]
pub struct AuthConfig {
    /// 这里使用 Vec
    ///
    /// 在编译规则时保证如果同一个路径下有多种公开方式时，采取最后指定的公开请求方法而非并集
    pub path_rules: Vec<PathRule>,

    pub jwt_encoder_config: JwtEncoderConfig,

    /// jwt 鉴权相关设置
    pub jwt_decoder_config: JwtDecoderConfig,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(deny_unknown_fields, default)]
pub struct StaticPathRule {
    /// 路径的通配符，UNIX shell 通配符
    pub pattern: String,

    /// 无需 token 即可访问的那些方法
    #[serde(default)]
    pub public_methods: Vec<HttpMethod>,
}

#[derive(Clone)]
pub struct PathRule {
    pub pattern: Pattern,
    pub public_methods: HashSet<HttpMethod>,
}

impl StaticAuthConfig {
    fn default_path_rules() -> Vec<StaticPathRule> {
        vec![StaticPathRule::default()]
    }
}

impl ConfigItem for StaticAuthConfig {
    type RuntimeConfig = AuthConfig;

    fn into_runtime(self) -> FatalResult<Self::RuntimeConfig> {
        let StaticAuthConfig {
            path_rules,
            jwt_encoder_config,
            jwt_decoder_config,
        } = self;

        let mut errors = MultiFatalError::new();

        let path_rules = path_rules
            .into_iter()
            .filter_map(|v| match v.into_runtime() {
                Ok(v) => Some(v),
                Err(mut e) => {
                    errors.append(&mut e);
                    None
                }
            })
            .collect();

        let (jwt_encoder_config, jwt_decoder_config) = (
            jwt_encoder_config.into_runtime(),
            jwt_decoder_config.into_runtime(),
        );

        match (jwt_encoder_config, jwt_decoder_config) {
            (Ok(jwt_encoder_config), Ok(jwt_decoder_config)) => Ok(AuthConfig {
                path_rules,
                jwt_encoder_config,
                jwt_decoder_config,
            }),
            (Err(mut e), Ok(_)) | (Ok(_), Err(mut e)) => {
                errors.append(&mut e);
                Err(errors)
            }
            (Err(mut e1), Err(mut e2)) => {
                errors.append(&mut e1).append(&mut e2);
                Err(errors)
            }
        }
    }
}

impl Default for StaticPathRule {
    fn default() -> Self {
        Self {
            pattern: "*".to_string(),
            public_methods: [HttpMethod::Safe].into(),
        }
    }
}

impl ConfigItem for StaticPathRule {
    type RuntimeConfig = PathRule;

    #[inline]
    fn into_runtime(self) -> FatalResult<Self::RuntimeConfig> {
        let StaticPathRule {
            pattern,
            public_methods,
        } = self;

        let pattern = Pattern::new(&pattern).map_err(|e| {
            let mut errors = MultiFatalError::new();
            errors.push(
                FatalError::from(e).when(format!("while parsing path rule pattern `{pattern}`")),
            );
            errors
        })?;

        let public_methods = public_methods.into_iter().collect();

        Ok(PathRule {
            pattern,
            public_methods,
        })
    }
}

impl PathRule {
    pub fn approved(&self, path: &str, method: HttpMethod) -> bool {
        self.pattern.matches(path) && self.public_methods.contains(&method)
    }
}
