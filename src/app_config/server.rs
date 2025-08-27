use std::{collections::HashSet, hash::Hash};

use base64::{prelude::BASE64_STANDARD, Engine};
use glob::Pattern;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::{error::cli::CliError, http::auth::HttpMethod};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ServerConfig {
    pub(super) port: u16,
    auth: AuthConfig,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct AuthConfig {
    /// 这里使用 HashSet 来保证如果同一个路径下有多种公开方式时，采取最后指定的公开请求方法而非并集
    #[serde(default)]
    pub path_rules: HashSet<PathRule>,

    /// jwt 鉴权相关设置
    #[serde(default, skip_serializing)]
    pub jwt_config: JwtConfig,
}

#[derive(Default, Serialize, Deserialize)]
pub struct PathRule {
    /// 路径的通配符，UNIX shell 通配符
    pub pattern: String,

    /// 无需 token 即可访问的那些方法
    #[serde(default)]
    pub public_methods: HashSet<HttpMethod>,
}

/// JWT 编解码的设置，这个是供中间件使用的，不会从配置文件中直接读取，而是使用 JwtConfigBuilder 构建
#[derive(Clone, Deserialize)]
#[serde(from = "JwtConfigBuilder", deny_unknown_fields)]
pub struct JwtConfig {
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
    pub header: Header,
    pub validation: Validation,
}

/// 这个就是配置文件的直接映射
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct JwtConfigBuilder {
    pub encoding_key: String,
    pub decoding_key: String,
    pub encode_algorithm: Algorithm,
    pub validation: ValidationConfig,
}

/// [`jsonwebtoken::Validation`] 没有实现 [`Deserialize`]，
///
/// 不能通过配置文件读取，于是就搞了这个中间层
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct ValidationConfig {
    required_spec_claims: Vec<String>,
    /// 延后过期
    leeway: u64,
    /// 提前过期
    reject_tokens_expiring_in_less_than: u64,
    validate_exp: bool,
    validate_nbf: bool,
    validate_aud: bool,
    aud: Option<Vec<String>>,
    iss: Option<Vec<String>>,
    sub: Option<String>,
    decode_algorithms: Vec<Algorithm>,
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
    pub fn path_rules(&self) -> &HashSet<PathRule> {
        &self.path_rules
    }

    pub fn jwt_config(&self) -> &JwtConfig {
        &self.jwt_config
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
                    "the PATH `{}` of path rules are not written in valid UNIX shell format, details: {e}",
                    self.pattern
                );
                None
            }
        }
    }
}

impl From<JwtConfigBuilder> for JwtConfig {
    fn from(value: JwtConfigBuilder) -> Self {
        value.build()
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        JwtConfigBuilder::default().build()
    }
}

impl Default for JwtConfigBuilder {
    fn default() -> Self {
        Self::new()
            .decode_key_from_base64_hmac("")
            .encode_key_from_base64_hmac("")
            .with_algorithm(Algorithm::HS256)
            .with_validation(ValidationConfig::default())
    }
}

impl JwtConfigBuilder {
    pub fn new() -> Self {
        Self {
            encoding_key: "".to_string(),
            decoding_key: "".to_string(),
            encode_algorithm: Algorithm::HS256,
            validation: ValidationConfig::default(),
        }
    }

    pub fn decode_key_from_base64_hmac(mut self, secret: &str) -> Self {
        self.decoding_key = secret.to_owned();
        self
    }

    pub fn encode_key_from_base64_hmac(mut self, secret: &str) -> Self {
        self.encoding_key = secret.to_owned();
        self
    }

    pub fn with_algorithm(mut self, algorithm: Algorithm) -> Self {
        self.encode_algorithm = algorithm;
        self
    }

    pub fn with_validation(mut self, validation: ValidationConfig) -> Self {
        self.validation = validation;
        self
    }

    pub fn build(self) -> JwtConfig {
        let decode_base64 = |secret, proc| {
            BASE64_STANDARD
                .decode(secret)
                .map_err(|e| CliError::from(e).with_source(proc).handle_strait_forward())
                .unwrap()
        };

        let encoding_key_bytes = decode_base64(
            self.encoding_key,
            "while decoding the encode secret key of jwt".to_string(),
        );

        let decoding_key_bytes = decode_base64(
            self.decoding_key,
            "while decoding the decode secret key of jwt".to_string(),
        );

        JwtConfig {
            encoding_key: EncodingKey::from_secret(&encoding_key_bytes),
            decoding_key: DecodingKey::from_secret(&decoding_key_bytes),
            header: Header::new(self.encode_algorithm),
            validation: self.validation.into(),
        }
    }
}

impl ValidationConfig {
    pub fn new(alg: Algorithm) -> Self {
        let required_claims =vec!["exp".into()];

        Self {
            required_spec_claims: required_claims,
            leeway: 0,
            reject_tokens_expiring_in_less_than: 0,

            validate_exp: true,
            validate_nbf: false,
            validate_aud: true,

            iss: None,
            aud: None,
            sub: None,

            decode_algorithms: vec![alg],
        }
    }
}

impl Default for ValidationConfig {
    fn default() -> Self {
        Self::new(Algorithm::default())
    }
}

impl From<ValidationConfig> for Validation {
    /// 执行配置文件到 [`Validation`] 的转化
    fn from(value: ValidationConfig) -> Self {
        let ValidationConfig {
            required_spec_claims,
            leeway,
            reject_tokens_expiring_in_less_than,
            validate_exp,
            validate_nbf,
            validate_aud,
            aud,
            iss,
            sub,
            decode_algorithms: algorithms,
        } = value;

        let mut validation = Validation::default();
        validation.required_spec_claims = required_spec_claims.into_iter().collect();
        validation.leeway = leeway;
        validation.reject_tokens_expiring_in_less_than = reject_tokens_expiring_in_less_than;
        validation.validate_exp = validate_exp;
        validation.validate_nbf = validate_nbf;
        validation.validate_aud = validate_aud;
        validation.aud = aud.map(|val| val.into_iter().collect());
        validation.iss = iss.map(|val| val.into_iter().collect());
        validation.sub = sub;
        validation.algorithms = algorithms;

        validation
    }
}
