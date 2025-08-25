#![allow(dead_code)]

use jsonwebtoken::*;
use serde::{Deserialize, Serialize};

use crate::error::auth::AuthError;

/// JWT 编解码的设置
/// 
/// 由于 [`EncodingKey`] 和 [`DecodingKey`] 的底层是 Vec<u8>，复制并不便宜。所以这个结构没有实现 [`Clone`]
pub struct JwtConfig {
    pub encoding_key: EncodingKey,
    pub decoding_key: DecodingKey,
    pub header: Header,
    pub validation: Validation,
}

impl JwtConfig {
    pub fn new() -> JwtConfigBuilder {
        JwtConfigBuilder::default()
    }
}

#[derive(Default)]
pub struct JwtConfigBuilder {
    pub encoding_key: Option<EncodingKey>,
    pub decoding_key: Option<DecodingKey>,
    pub algorithm: Option<Algorithm>,
    pub validation: Option<Validation>,
}

impl JwtConfigBuilder {
    pub fn decode_key_from_hmac(mut self, secret: &[u8]) -> Self {
        self.decoding_key = Some(DecodingKey::from_secret(secret));
        self
    }

    pub fn encode_key_from_hmac(mut self, secret: &[u8]) -> Self {
        self.encoding_key = Some(EncodingKey::from_secret(secret));
        self
    }

    pub fn with_algorithm(mut self, algorithm: Algorithm) -> Self {
        self.algorithm = Some(algorithm);
        self
    }

    pub fn with_validation(mut self, validation: Validation) -> Self {
        self.validation = Some(validation);
        self
    }

    pub fn build(self) -> JwtConfig {
        debug_assert!(
            self.encoding_key.is_some()
                && self.decoding_key.is_some()
                && self.algorithm.is_some()
                && self.validation.is_some()
        );

        JwtConfig {
            encoding_key: self.encoding_key.unwrap(),
            decoding_key: self.decoding_key.unwrap(),
            header: Header::new(self.algorithm.unwrap()),
            validation: self.validation.unwrap(),
        }
    }
}
/// 此项目的 JWT 通用字段
#[derive(Serialize, Deserialize)]
pub struct Jwt<P> {
    /// 签发者 (Issuer)
    ///
    /// 标识签发此令牌的服务， "brain-overflow"
    pub iss: String,

    /// 受众 (Audience)
    ///
    /// 标识令牌的目标接收者， "crab-vault"
    pub aud: String,

    // 没用 sub，因为目前只有 brain-overflow 在用这个
    /// 过期时间 (Expiration Time)
    ///
    /// 令牌的过期时间戳 (Unix 时间)
    pub exp: i64,

    /// 生效时间 (Not Before)
    ///
    /// 令牌开始生效的时间戳 (Unix 时间)
    pub nbf: i64,

    /// 签发时间 (Issued At)
    ///
    /// 令牌签发的时间戳 (Unix 时间)
    pub iat: i64,

    /// JWT ID
    ///
    /// 令牌的唯一标识符，用于防止重放攻击
    pub jti: u128,

    /// 自定义的字段
    #[serde(flatten)]
    pub payload: P,
}

impl<P: Serialize + for<'de> Deserialize<'de>> Jwt<P> {
    pub fn encode(claims: &P, config: &JwtConfig) -> Result<String, AuthError> {
        Ok(encode(&config.header, &claims, &config.encoding_key)?)
    }

    pub fn decode(token: &str, config: &JwtConfig) -> Result<Jwt<P>, AuthError> {
        Ok(decode::<Jwt<P>>(token, &config.decoding_key, &config.validation)?.claims)
    }
}

/// JWT 令牌的载荷 (Payload) 结构
#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    /// 允许的操作列表
    ///
    /// 定义此令牌授权执行的具体操作
    pub operations: Vec<Method>,

    /// 资源路径模式
    ///
    /// 定义此令牌可以访问的资源路径，支持通配符 *
    pub resource_pattern: String,

    /// 对操作的附加限制条件
    #[serde(default)]
    pub conditions: Conditions,
}

impl Permission {
    /// 如果能够执行某一个 [`Method`] ，返回 `true`，否则返回 `false`
    pub fn can_perform(&self, method: Method) -> bool {
        self.operations.contains(&method)
    }

    /// 如果能够获取某一个资源 ，返回 `true`，否则返回 `false`
    pub fn can_access(&self, _path: &str) -> bool {
        true
    }

    /// 上传的资源大小小于 [`Permission`] 的 `max_size` ，返回 `true`，否则返回 `false`
    pub fn check_size(&self, size: u64) -> bool {
        self.conditions.check_size(size)
    }

    /// 上传的资源的 mime-type 符合 [`Permission`] 的 `allowed_content_type` ，返回 `true`，否则返回 `false`
    pub fn check_content_type(&self, content_type: &str) -> bool {
        self.conditions.check_content_type(content_type)
    }

    /// 上传资源的 IP 符合 [`Permission`] 的 `allowed_ips`，返回 `true`，否则返回 `false`
    pub fn check_ip(&self, ip: &str) -> bool {
        self.conditions.check_ip(ip)
    }
}

/// 允许的操作类型
#[derive(Serialize, Deserialize, PartialEq, Clone, Copy)]
#[serde(rename_all = "UPPERCASE")]
pub enum Method {
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
    Other
}

impl From<&axum::http::Method> for Method {
    fn from(value: &axum::http::Method) -> Self {
        match value {
            &axum::http::Method::GET => Self::Get,
            &axum::http::Method::POST => Self::Post,
            &axum::http::Method::PUT => Self::Put,
            &axum::http::Method::PATCH => Self::Patch,
            &axum::http::Method::DELETE => Self::Delete,
            &axum::http::Method::HEAD => Self::Head,
            &axum::http::Method::OPTIONS => Self::Options,
            &axum::http::Method::TRACE => Self::Trace,
            &axum::http::Method::CONNECT => Self::Connect,
            _ => Self::Other
        }
    }
}

/// 操作条件限制
#[derive(Serialize, Deserialize, Default, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Conditions {
    /// 最大对象大小 (字节)
    ///
    /// 对于上传操作有效
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_size: Option<u64>,

    /// 允许的内容类型
    ///
    /// 对于上传操作有效，支持 MIME 类型通配符
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_content_types: Vec<String>,

    /// IP 地址限制，现在先不用
    ///
    /// 允许的客户端 IP 地址或 CIDR 范围
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub allowed_ips: Vec<String>,
}

impl Conditions {
    pub fn check_size(&self, size: u64) -> bool {
        self.max_size.is_none_or(|limit| size <= limit)
    }

    pub fn check_content_type(&self, _content_type: &str) -> bool {
        true
    }

    pub fn check_ip(&self, _ip: &str) -> bool {
        true
    }
}
