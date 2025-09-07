//! # JSON Web Token (JWT) Authentication Library
//!
//! 这是一个基于 `jsonwebtoken` crate 封装的 JWT 认证模块。
//! 它提供了灵活的 JWT 生成、解码、验证以及基于声明的权限控制功能。
//!
//! ## 主要特性
//!
//! - **强类型 JWT**: 通过泛型 `Jwt<P>` 支持自定义的载荷 (Payload)。
//! - **流畅的构建器模式**: 链式调用方法轻松构建 JWT 声明。
//! - **灵活的配置**: 支持多种签名算法，便于密钥轮换。
//! - **集成的错误处理**: `AuthError` 枚举与 `axum` 的 `IntoResponse` 无缝集成。
//! - **基于声明的权限**: 内置 `Permission` 结构，支持基于 Glob 模式的资源和操作权限控制。
//!
//! ## 快速上手
//!
//! ```rust,ignore
//! // 1. 定义你的载荷
//! #[derive(Serialize, Deserialize)]
//! struct MyPayload {
//!     user_id: u32,
//!     permissions: Vec<Permission>,
//! }
//!
//! // 2. 创建配置
//! let config = JwtConfig {
//!     // ... 设置 encoding_key, decoding_key, header, validation
//! };
//!
//! // 3. 创建并签发 Token
//! let payload = MyPayload { /* ... */ };
//! let claims = Jwt::new(payload)
//!     .issue_as("my-app")
//!     .expires_in(chrono::Duration::hours(1));
//!
//! let token = Jwt::encode(&claims, &config)?;
//!
//! // 4. 在 Axum Handler 中解码和验证
//! async fn protected_route(
//!     State(config): State<Arc<JwtConfig>>,
//!     TypedHeader(Authorization(bearer)): TypedHeader<Authorization<Bearer>>,
//! ) -> Result<Json<MyPayload>, AuthError> {
//!     let decoded = Jwt::decode::<MyPayload>(bearer.token(), &config)?;
//!     Ok(Json(decoded.payload))
//! }
//! ```

pub mod error;

use std::{collections::HashMap, sync::Arc};

use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use clap::ValueEnum;
use glob::Pattern;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::error::AuthError;

/// JWT 编解码所需的完整配置。
///
/// 建议在应用启动时创建此结构，并使用 `Arc` 在多个线程间共享。
pub struct JwtConfig {
    /// 用于签发 JWT 的密钥。
    pub encoding_key: EncodingKey,

    /// 用于验证 JWT 的密钥映射。
    ///
    /// `HashMap` 的键是签名算法 (`Algorithm`)，值是对应的解码密钥 (`DecodingKey`)。
    /// 这允许应用同时支持多种算法，例如在进行密钥轮换或算法迁移时。
    pub decoding_key: HashMap<Algorithm, DecodingKey>,

    /// JWT 的头部 (`Header`)。定义了所使用的算法 (`alg`) 和类型 (`typ`, 通常是 "JWT")。
    pub header: Header,

    /// JWT 的验证规则。
    ///
    /// 用于配置如何验证 `exp`, `nbf`, `iss`, `aud` 等标准声明。
    pub validation: Validation,
}

/// 表示一个完整的 JWT，包含标准声明和自定义载荷。
///
/// 泛型参数 `P` 代表自定义的载荷 (Payload) 结构体。
#[derive(Serialize, Deserialize, Debug)]
pub struct Jwt<P> {
    /// (Issuer) 签发者。可选。
    pub iss: Option<String>,

    /// (Audience) 受众。可以是一个或多个，没有也行。
    pub aud: Vec<String>,

    /// (Expiration Time) 过期时间。Unix 时间戳。
    pub exp: i64,

    /// (Not Before) 生效时间。Unix 时间戳。
    pub nbf: i64,

    /// (Issued At) 签发时间。Unix 时间戳。
    pub iat: i64,

    /// (JWT ID) 令牌唯一标识。
    pub jti: Uuid,

    /// 自定义的载荷数据。
    /// `#[serde(flatten)]` 会将 `payload` 的字段直接嵌入到 JWT 的顶层。
    #[serde(flatten)]
    pub payload: P,
}

/// JWT 令牌的载荷 (Payload) 中用于权限控制的部分。
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    /// 允许的操作列表。
    ///
    /// 定义此令牌授权执行的具体 HTTP 方法。
    pub operations: Vec<HttpMethod>,

    /// 资源路径模式。
    ///
    /// 定义此令牌可以访问的资源路径，支持通配符 `*` 和 `?` (Glob 模式)。
    /// 例如: `/users/*` 或 `/files/???.txt`。
    pub resource_pattern: Option<String>,

    /// 允许上传的最大对象大小 (字节)。
    ///
    /// `None` 表示没有限制。
    pub max_size: Option<u64>,

    /// 允许的内容类型 (MIME types)。
    ///
    /// 支持通配符，例如 `image/*` 或 `*`。
    pub allowed_content_types: Vec<String>,
}

/// HTTP 操作方法枚举。
///
/// `ValueEnum` 用于 `clap` 集成，使其可以在命令行参数中使用。
#[derive(Serialize, Deserialize, PartialEq, Eq, Hash, Clone, Copy, Debug, ValueEnum)]
#[serde(rename_all = "UPPERCASE")]
pub enum HttpMethod {
    Get,
    Post,
    Put,
    Patch,
    Delete,
    Head,
    Options,
    Trace,
    Connect,
    /// 代表非标准的 HTTP 方法。
    Other,
    /// 代表所有 HTTP 方法，通常用于管理员权限。
    All,
}

impl<P: Serialize + for<'de> Deserialize<'de>> Jwt<P> {
    /// 使用给定的配置将 JWT 声明编码为字符串形式的 Token。
    #[inline]
    pub fn encode(claims: &Jwt<P>, config: &JwtConfig) -> Result<String, AuthError> {
        Ok(jsonwebtoken::encode(
            &config.header,
            claims,
            &config.encoding_key,
        )?)
    }

    /// 使用给定的配置解码并验证一个字符串形式的 Token。
    ///
    /// 此函数会执行完整的验证流程，包括：
    /// 1. 检查签名是否有效。
    /// 2. 验证 `exp` 和 `nbf` 时间戳。
    /// 3. 根据 `config.validation` 中的设置验证 `iss` 和 `aud`。
    pub fn decode(token: &str, config: &JwtConfig) -> Result<Jwt<P>, AuthError> {
        let header = jsonwebtoken::decode_header(token)?;
        let key = config
            .decoding_key
            .get(&header.alg)
            .ok_or(AuthError::InvalidAlgorithm(header.alg))?;
        Ok(jsonwebtoken::decode::<Jwt<P>>(token, key, &config.validation)?.claims)
    }

    /// **[不安全]** 在不验证签名的情况下解码 JWT 的载荷。
    ///
    /// # 警告
    ///
    /// **绝对不要**相信此函数返回的数据！因为它**没有验证** JWT 的签名。
    /// 这意味着任何人都可以伪造这个 JWT 的内容。
    ///
    /// 此函数仅应用于需要查看 Token 内容的调试或日志记录场景。
    /// 在任何与安全相关的逻辑中，都**必须**使用 [`Jwt::decode`]。
    pub fn decode_unchecked(token: &str) -> Result<serde_json::Value, AuthError> {
        let mut parts = token.split('.');
        let _header = parts.next();
        let payload = parts.next().ok_or(AuthError::TokenInvalid)?;

        let decoded_payload = BASE64_STANDARD_NO_PAD.decode(payload)?;
        let json_value = serde_json::from_slice(&decoded_payload).map_err(Arc::new)?;

        Ok(json_value)
    }

    /// 创建一个新的 `Jwt` 实例，并填入默认值。
    ///
    /// 默认值:
    /// - `iss`: `None`
    /// - `aud`: 空 `Vec`
    /// - `exp`: `i32::MAX` (永不过期, 其实是 UNIX 时间戳能表示的上限)
    /// - `nbf`: `0` (立即生效)
    /// - `iat`: 当前时间的 Unix 时间戳
    /// - `jti`: 一个使用 [`Uuid::new_v4`] 新生成的 [`Uuid`]
    #[inline]
    pub fn new(payload: P) -> Self {
        Self {
            iss: None,
            aud: vec![],
            exp: i32::MAX as i64,
            nbf: 0,
            iat: chrono::Utc::now().timestamp(),
            jti: Uuid::new_v4(),
            payload,
        }
    }

    /// 设置 JWT 的签发者 (`iss`)。
    #[inline]
    pub fn issue_as<T>(mut self, iss: T) -> Self
    where
        T: Into<String>,
    {
        self.iss = Some(iss.into());
        self
    }

    /// 设置 JWT 的签发者 (`iss`)，接受一个 [`Option<T: Into<String>>`]。
    #[inline]
    pub fn issue_as_option<T>(mut self, iss: Option<T>) -> Self
    where
        T: Into<String>,
    {
        self.iss = iss.map(|val| val.into());
        self
    }

    /// 设置 JWT 的受众 (`aud`)。
    #[inline]
    pub fn audiences<'a, T>(mut self, aud: &'a [T]) -> Self
    where
        String: From<&'a T>,
    {
        self.aud = aud.iter().map(|aud| aud.into()).collect();
        self
    }

    /// 设置 JWT 的受众 (`aud`)，接受一个 `Option`。
    #[inline]
    pub fn audiences_option<'a, T>(mut self, aud: Option<&'a [T]>) -> Self
    where
        String: From<&'a T>,
    {
        self.aud = aud
            .map(|aud| aud.iter().map(String::from).collect())
            .unwrap_or_default();
        self
    }

    /// 设置 JWT 的过期时间，从现在开始计算。
    #[inline]
    pub fn expires_in(mut self, duration: chrono::Duration) -> Self {
        self.exp = (chrono::Utc::now() + duration).timestamp();
        self
    }

    /// 设置 JWT 的生效时间，从现在开始计算。
    #[inline]
    pub fn not_valid_in(mut self, duration: chrono::Duration) -> Self {
        self.nbf = (chrono::Utc::now() + duration).timestamp();
        self
    }

    /// 设置 JWT 的过期时间为一个绝对的时间点。
    #[inline]
    pub fn expires_at<T>(mut self, when: chrono::DateTime<T>) -> Self
    where
        T: chrono::TimeZone,
    {
        self.exp = when.timestamp();
        self
    }

    /// 设置 JWT 的生效时间为一个绝对的时间点。
    #[inline]
    pub fn not_valid_till<T>(mut self, when: chrono::DateTime<T>) -> Self
    where
        T: chrono::TimeZone,
    {
        self.nbf = when.timestamp();
        self
    }

    /// 在构建 token 的时候更换 uuid
    #[inline]
    pub fn uuid(mut self, id: Uuid) -> Self {
        self.jti = id;
        self
    }
}

impl Permission {
    /// 创建一个 <u>**拥有所有权限**</u> 的 `root` `Permission`。
    ///
    /// ### 这个操作应当尽量少用，因为这个获取这个权限就意味着该用户能够读写所有的资源 (所有！)
    ///
    /// - 允许操作: [`HttpMethod::All`]
    /// - 允许资源: [`Some("*".to_string())`](Some) (所有路径)
    /// - 大小限制：[`None`]
    /// - MIME: **所有**
    pub fn new_root() -> Self {
        Self {
            operations: vec![HttpMethod::All],
            resource_pattern: Some("*".to_string()),
            max_size: None,
            allowed_content_types: vec!["*".to_string()],
        }
    }

    /// 创建一个 <u>**没有任何权限**</u> 的 "minimum" `Permission`。
    ///
    /// 直接签发这个 [`Permission`] 将导致完全无法访问任何内容
    ///
    /// - 允许操作: 无
    /// - 允许资源: [`None`] (所有路径都不允许)
    /// - 大小限制：[`Some(0)`](Some) (上传的最大包问题大小为 0 字节)
    /// - MIME: **所有都不行**
    pub fn new_minimum() -> Self {
        Self {
            operations: vec![],
            resource_pattern: None,
            max_size: Some(0),
            allowed_content_types: vec![],
        }
    }

    /// 更换这个 [`Permission`] 允许的 operations
    ///
    /// 注意这会**更换**，而不是添加
    #[inline]
    pub fn permit_method(mut self, methods: Vec<HttpMethod>) -> Self {
        self.operations = methods;
        self
    }

    /// 修改这个令牌能够访问的资源路径
    #[inline]
    pub fn permit_access_url<T>(mut self, pattern: T) -> Self
    where
        T: Into<String>,
    {
        self.resource_pattern = Some(pattern.into());
        self
    }

    /// 修改这个令牌能够访问的资源路径
    #[inline]
    pub fn permit_access_url_option<T>(mut self, pattern: Option<T>) -> Self
    where
        T: Into<String>,
    {
        self.resource_pattern = pattern.map(T::into);
        self
    }

    /// 设置最大的内容长度
    #[inline]
    pub fn restrict_maximum_size(mut self, max: u64) -> Self {
        self.max_size = Some(max);
        self
    }

    #[inline]
    pub fn restrict_maximum_size_option(mut self, max: Option<u64>) -> Self {
        self.max_size = max;
        self
    }

    /// 此令牌允许的最大内容类型
    #[inline]
    pub fn permit_content_type(mut self, content_type: Vec<String>) -> Self {
        self.allowed_content_types = content_type;
        self
    }

    /// 检查此权限是否允许执行给定的 HTTP 方法。
    ///
    /// 如果 `operations` 包含 `HttpMethod::All` 或指定的 `method`，则返回 `true`。
    pub fn can_perform_method<T>(&self, method: T) -> bool
    where
        T: Into<HttpMethod>,
    {
        self.operations.contains(&HttpMethod::All) || self.operations.contains(&method.into())
    }

    /// 检查此权限是否能访问给定的资源路径。
    ///
    /// 使用 `resource_pattern` 对 `path` 进行 Glob 匹配。
    /// 如果 `resource_pattern` 不是一个有效的 Glob 模式，会安全地返回 `false`。
    pub fn can_access(&self, path: &str) -> bool {
        match &self.resource_pattern {
            Some(pat) => Pattern::new(pat)
                .map(|pattern| pattern.matches(path))
                .unwrap_or(false),
            None => false,
        }
    }

    /// 检查给定的大小是否在 `max_size` 的限制内。
    ///
    /// 如果 `max_size` 是 `None` (无限制)，或者 `size` 小于等于限制，则返回 `true`。
    pub fn check_size(&self, size: u64) -> bool {
        self.max_size.is_none_or(|limit| size <= limit)
    }

    /// 检查给定的内容类型是否被允许。
    ///
    /// 遍历 `allowed_content_types`，对每个模式进行 Glob 匹配。
    ///
    /// 由于这个 [`Conditions`] 通常来说是嵌入到每一个 token 中的，这就注定了无法缓存，或者说缓存意义不大
    ///
    /// 并且通常来说，一个许可证的内容类型条件应当比较简洁
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

impl From<axum::http::Method> for HttpMethod {
    fn from(value: axum::http::Method) -> Self {
        match value {
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
