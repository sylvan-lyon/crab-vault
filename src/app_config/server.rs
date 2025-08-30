use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use base64::{Engine, prelude::BASE64_STANDARD};
use clap::error::ErrorKind;
use glob::Pattern;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

use crate::{
    error::cli::{CliError, MultiCliError},
    http::auth::HttpMethod,
};

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields, default)]
pub struct ServerConfig {
    pub(super) port: u16,
    auth: AuthConfig,
}

#[derive(Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields, default)]
pub struct AuthConfig {
    /// 这里使用 Vec
    ///
    /// 在编译规则时保证如果同一个路径下有多种公开方式时，采取最后指定的公开请求方法而非并集
    #[serde(default)]
    pub path_rules: Vec<PathRule>,

    /// jwt 鉴权相关设置
    #[serde(default, skip_serializing)]
    pub jwt_config: JwtConfig,
}

#[derive(Default, Serialize, Deserialize, Clone)]
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
    pub decoding_key: HashMap<Algorithm, DecodingKey>,
    pub header: Header,
    pub validation: Validation,
}

/// 这个就是配置文件的直接映射
#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct JwtConfigBuilder {
    pub encoding: AlgKeyPair,
    pub decoding: Vec<AlgKeyPair>,
    pub validation: ValidationConfig,
}

#[derive(Serialize, Deserialize, Default, Clone)]
pub struct AlgKeyPair {
    algorithm: Algorithm,
    form: KeyForm,
    #[serde(alias = "path")]
    key: String,
}

#[derive(Serialize, Deserialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum KeyForm {
    #[default]
    DerInline,
    DerFile,
    PemInline,
    PemFile,
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
    pub fn jwt_config(&self) -> &JwtConfig {
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

impl From<JwtConfigBuilder> for JwtConfig {
    fn from(value: JwtConfigBuilder) -> Self {
        value.build().map_err(|e| e.exit_now()).unwrap()
    }
}

impl Default for JwtConfig {
    fn default() -> Self {
        JwtConfigBuilder::default()
            .build()
            .map_err(|e| e.exit_now())
            .unwrap()
    }
}

impl Default for JwtConfigBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl JwtConfigBuilder {
    pub fn new() -> Self {
        Self {
            encoding: AlgKeyPair::default(),
            decoding: vec![AlgKeyPair::default()],
            validation: ValidationConfig::default(),
        }
    }

    pub fn build(self) -> Result<JwtConfig, MultiCliError> {
        let mut errors = MultiCliError::new();

        let decoding_key = self
            .decoding
            .iter()
            .filter_map(|pair| match pair.build_as_decode_key() {
                Ok(alg_key_pair) => Some(alg_key_pair),
                Err(e) => {
                    errors.add(e);
                    None
                }
            })
            .collect();

        let encoding_key = match self.encoding.build_as_encode_key() {
            Ok(alg_key_pair) => alg_key_pair.1,
            Err(e) => {
                errors.add(e);
                return Err(errors);
            }
        };

        if !errors.is_empty() {
            return Err(errors);
        }

        let res = JwtConfig {
            encoding_key,
            decoding_key,
            header: Header::new(self.encoding.algorithm),
            validation: self.validation.into(),
        };

        if !res.decoding_key.contains_key(&self.encoding.algorithm) {
            tracing::warn!(
                "no decoding key provided for encoding algorithm {:?}; tokens signed by this server might not be verifiable",
                self.encoding.algorithm
            );
        }

        Ok(res)
    }
}

impl AlgKeyPair {
    fn get_key(&self) -> Result<Vec<u8>, CliError> {
        let res = match self.form {
            KeyForm::DerInline => BASE64_STANDARD.decode(self.key.clone()).map_err(|e| {
                CliError::from(e).add_source(format!(
                    "while decoding the given jwt secrete key `{}` into binary form",
                    self.key
                        .get(0..4)
                        .and_then(|val| Some(format!("{val}...")))
                        .unwrap_or(self.key.clone())
                ))
            })?,
            KeyForm::DerFile => std::fs::read(&self.key).map_err(|e| {
                CliError::from(e).add_source(format!("while reading the der key from {}", self.key))
            })?,
            KeyForm::PemInline => self.key.clone().into_bytes(),
            KeyForm::PemFile => std::fs::read(&self.key).map_err(|e| {
                CliError::from(e).add_source(format!("while reading the pem key from {}", self.key))
            })?,
        };

        if res.len() < 32 {
            tracing::warn!("the secret key is too short to prevent brute force cracking")
        }

        Ok(res)
    }

    fn build_as_encode_key(&self) -> Result<(Algorithm, EncodingKey), CliError> {
        if self.form.is_der() {
            let build_from_der = match self.algorithm {
                Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => EncodingKey::from_secret,
                Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => EncodingKey::from_rsa_der,
                Algorithm::PS256 | Algorithm::PS384 | Algorithm::PS512 => EncodingKey::from_rsa_der,
                Algorithm::ES256 | Algorithm::ES384 => EncodingKey::from_ec_der,
                Algorithm::EdDSA => EncodingKey::from_ed_der,
            };

            Ok((
                self.algorithm,
                build_from_der(&self.get_key().map_err(|e| {
                    e.add_source("while building jwt encoding key from a der form".into())
                })?),
            ))
        } else if self.form.is_pem() {
            let build_from_pem = match self.algorithm {
                Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                    return Err(CliError::new(
                        ErrorKind::Io,
                        format!(
                            "cannot use a .pem file `{}` to store a hmac secret key, you should use a .der file or a inline base64 encoded string",
                            self.key
                        ),
                        Some("while building jwt encoding key from a der form".into()),
                    ));
                }
                Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => EncodingKey::from_rsa_pem,
                Algorithm::PS256 | Algorithm::PS384 | Algorithm::PS512 => EncodingKey::from_rsa_pem,
                Algorithm::ES256 | Algorithm::ES384 => EncodingKey::from_ec_pem,
                Algorithm::EdDSA => EncodingKey::from_ed_pem,
            };

            Ok((
                self.algorithm,
                build_from_pem(&self.get_key().map_err(|e| {
                    e.add_source("while building jwt encoding key from a pem form".into())
                })?)
                .map_err(|e| {
                    CliError::new(
                        ErrorKind::Io,
                        e.to_string(),
                        Some("while building jwt encoding key from a pem form".into()),
                    )
                })?,
            ))
        } else {
            unreachable!(
                "Sylvan, 你加了新的变体但是没有添加相应的条件判断，去检查你的 is_der 和 is_pem 方法是否包含了所有的情况"
            )
        }
    }

    fn build_as_decode_key(&self) -> Result<(Algorithm, DecodingKey), CliError> {
        if self.form.is_der() {
            let build_from_der = match self.algorithm {
                Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => DecodingKey::from_secret,
                Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => DecodingKey::from_rsa_der,
                Algorithm::PS256 | Algorithm::PS384 | Algorithm::PS512 => DecodingKey::from_rsa_der,
                Algorithm::ES256 | Algorithm::ES384 => DecodingKey::from_ec_der,
                Algorithm::EdDSA => DecodingKey::from_ed_der,
            };

            Ok((
                self.algorithm,
                build_from_der(&self.get_key().map_err(|e| {
                    e.add_source("while building jwt encoding key from a der form".into())
                })?),
            ))
        } else if self.form.is_pem() {
            let build_from_pem = match self.algorithm {
                Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
                    return Err(CliError::new(
                        ErrorKind::Io,
                        format!(
                            "cannot use a .pem file `{}` to store a hmac secret key, you should use a .der file or a inline base64 encoded string",
                            self.key
                        ),
                        Some("while building jwt decoding key from a pem form".into()),
                    ));
                }
                Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => DecodingKey::from_rsa_pem,
                Algorithm::PS256 | Algorithm::PS384 | Algorithm::PS512 => DecodingKey::from_rsa_pem,
                Algorithm::ES256 | Algorithm::ES384 => DecodingKey::from_ec_pem,
                Algorithm::EdDSA => DecodingKey::from_ed_pem,
            };

            Ok((
                self.algorithm,
                build_from_pem(&self.get_key()?).map_err(|e| {
                    CliError::new(
                        ErrorKind::Io,
                        e.to_string(),
                        Some("while building jwt decoding key from a pem form".into()),
                    )
                })?,
            ))
        } else {
            unreachable!(
                "Sylvan, 你加了新的变体但是没有添加相应的条件判断，去检查你的 is_der 和 is_pem 方法是否包含了所有的情况"
            )
        }
    }
}

impl KeyForm {
    fn is_der(&self) -> bool {
        matches!(self, KeyForm::DerInline | KeyForm::DerFile)
    }

    fn is_pem(&self) -> bool {
        matches!(self, KeyForm::PemInline | KeyForm::PemFile)
    }
}

impl ValidationConfig {
    pub fn new(alg: Algorithm) -> Self {
        let required_claims = vec!["exp".into()];

        Self {
            required_spec_claims: required_claims,
            leeway: 0,
            reject_tokens_expiring_in_less_than: 0,

            validate_exp: true,
            validate_nbf: false,

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
        validation.validate_aud = matches!(&aud, Some(val) if !val.is_empty());
        validation.aud = aud.map(|val| val.into_iter().collect());
        validation.iss = iss.map(|val| val.into_iter().collect());
        validation.sub = sub;
        validation.algorithms = algorithms;

        validation
    }
}
