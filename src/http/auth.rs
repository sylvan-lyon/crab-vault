use clap::ValueEnum;
use glob::Pattern;
use jsonwebtoken::*;
use serde::{Deserialize, Serialize};

use crate::{app_config::server::JwtConfig, error::auth::AuthError};

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
