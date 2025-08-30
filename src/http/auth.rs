use std::collections::HashMap;

use base64::{Engine, prelude::BASE64_STANDARD};
use clap::{ValueEnum, error::ErrorKind};
use glob::Pattern;
use jsonwebtoken::*;
use serde::{Deserialize, Serialize};

use crate::error::{
    auth::AuthError,
    cli::{CliError, MultiCliError},
};

/// JWT 编解码的设置，这个是供中间件使用的，不会从配置文件中直接读取，而是使用 JwtConfigBuilder 构建
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
    leeway: u64,
    reject_tokens_expiring_in_less_than: u64,
    validate_exp: bool,
    validate_nbf: bool,
    aud: Option<Vec<String>>,
    iss: Option<Vec<String>>,
    sub: Option<String>,
    decode_algorithms: Vec<Algorithm>,
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
                        .map(|val| format!("{val}..."))
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

/// 此项目的 JWT 通用字段
#[derive(Serialize, Deserialize)]
pub struct Jwt<P> {
    pub iss: Option<String>,
    pub aud: Vec<String>,
    pub exp: i64,
    pub nbf: i64,
    pub iat: i64,
    pub jti: u128,

    #[serde(flatten)]
    pub payload: P,
}

/// JWT 令牌的载荷 (Payload) 结构
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    /// 允许的操作列表
    ///
    /// 定义此令牌授权执行的具体操作
    pub operations: Vec<HttpMethod>,

    /// 资源路径模式
    ///
    /// 定义此令牌可以访问的资源路径，支持通配符 *
    pub resource_pattern: String,

    /// 对操作的附加限制条件
    #[serde(default)]
    pub conditions: Conditions,
}

/// 允许的操作类型
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy, ValueEnum)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    // 中间件实现
    Options,
    Trace,
    Connect,
    // 其他
    Other,
    // 以上所有
    All,
}

/// 操作条件限制
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Conditions {
    /// 最大对象大小 (字节)
    ///
    /// 对于上传操作有效
    pub max_size: Option<u64>,

    /// 允许的内容类型
    ///
    /// 对于上传操作有效，支持 MIME 类型通配符
    pub allowed_content_types: Vec<String>,
}

impl<P: Serialize + for<'de> Deserialize<'de>> Jwt<P> {
    pub fn encode(claims: &Jwt<P>, config: &JwtConfig) -> Result<String, AuthError> {
        Ok(encode(&config.header, claims, &config.encoding_key)?)
    }

    pub fn decode(token: &str, config: &JwtConfig) -> Result<Jwt<P>, AuthError> {
        let header = jsonwebtoken::decode_header(token)?;
        let key = config
            .decoding_key
            .get(&header.alg)
            .ok_or(AuthError::InvalidAlgorithm(header.alg))?;
        Ok(decode::<Jwt<P>>(token, key, &config.validation)?.claims)
    }

    pub fn decode_unchecked(token: &str, config: &JwtConfig) -> Result<Jwt<P>, AuthError> {
        let header = jsonwebtoken::decode_header(token)?;
        let key = config
            .decoding_key
            .get(&header.alg)
            .ok_or(AuthError::InvalidAlgorithm(header.alg))?;
        let mut validation = Validation::new(header.alg);
        validation.validate_aud = false;
        validation.validate_exp = false;
        validation.validate_nbf = false;
        validation.insecure_disable_signature_validation();
        Ok(decode::<Jwt<P>>(token, key, &validation)?.claims)
    }
}

impl Permission {
    pub fn new_root() -> Self {
        Self {
            operations: vec![HttpMethod::All],
            resource_pattern: "*".to_string(),
            conditions: Conditions {
                max_size: None,
                allowed_content_types: vec!["*".to_string()],
            },
        }
    }

    /// 如果能够执行某一个 [`Method`] ，返回 `true`，否则返回 `false`
    pub fn can_perform(&self, method: HttpMethod) -> bool {
        self.operations.contains(&HttpMethod::All) || self.operations.contains(&method)
    }

    /// 如果能够获取某一个资源 ，返回 `true`，否则返回 `false`
    ///
    /// 如果签名中的 `resource_pattern` 字段不是一个 UNIX Shell Pattern 的话，会直接返回 false
    pub fn can_access(&self, path: &str) -> bool {
        Pattern::new(&self.resource_pattern)
            .map(|pattern| pattern.matches(path))
            .unwrap_or(false)
    }

    /// 上传的资源大小小于 [`Permission`] 的 `max_size` ，返回 `true`，否则返回 `false`
    pub fn check_size(&self, size: u64) -> bool {
        self.conditions.check_size(size)
    }

    /// 上传的资源的 mime-type 符合 [`Permission`] 的 `allowed_content_type` ，返回 `true`，否则返回 `false`
    pub fn check_content_type(&self, content_type: &str) -> bool {
        self.conditions.check_content_type(content_type)
    }
}

impl From<&axum::http::Method> for HttpMethod {
    fn from(value: &axum::http::Method) -> Self {
        match *value {
            axum::http::Method::GET => Self::Get,
            axum::http::Method::POST => Self::Post,
            axum::http::Method::PUT => Self::Put,
            axum::http::Method::PATCH => Self::Patch,
            axum::http::Method::DELETE => Self::Delete,
            axum::http::Method::HEAD => Self::Head,
            axum::http::Method::OPTIONS => Self::Options,
            axum::http::Method::TRACE => Self::Trace,
            axum::http::Method::CONNECT => Self::Connect,
            _ => Self::Other,
        }
    }
}

impl Default for Conditions {
    /// 默认情况下，不允许任何的请求报文体的大小超过 0 字节，或者说，默认不允许上传内容
    fn default() -> Self {
        Self {
            max_size: Some(0),
            allowed_content_types: vec![],
        }
    }
}

impl Conditions {
    pub fn check_size(&self, size: u64) -> bool {
        self.max_size.is_none_or(|limit| size <= limit)
    }

    pub fn check_content_type(&self, content_type: &str) -> bool {
        for allows in &self.allowed_content_types {
            if Pattern::new(allows)
                .map(|e| e.matches(content_type))
                .unwrap_or(false)
            {
                return true;
            }
        }

        false
    }
}
