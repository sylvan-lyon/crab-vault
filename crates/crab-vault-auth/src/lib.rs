pub mod error;

use clap::ValueEnum;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::vec;
use uuid::Uuid;
use validator::{Validate, ValidationError};

#[cfg(feature = "server-side")]
use base64::Engine;
#[cfg(feature = "server-side")]
use glob::Pattern;
#[cfg(feature = "server-side")]
use jsonwebtoken::{DecodingKey, Validation};

use crate::error::AuthError;

pub struct JwtEncoder {
    /// 用于签发 JWT 的密钥。从 kid 到 ([`EncodingKey`], [`Algorithm`]) 的映射
    pub encoding_key: HashMap<String, (EncodingKey, Algorithm)>,

    kids: Vec<String>
}

#[cfg(feature = "server-side")]
pub struct JwtDecoder {
    /// 用于验证 JWT 的密钥映射。
    ///
    /// [`HashMap`] 的键是签发者 (iss, kid)，值是对应的轮换密钥 ([`DecodingKey`])。
    #[cfg(feature = "server-side")]
    decoding_keys: HashMap<(String, String), DecodingKey>,

    /// JWT 的验证规则。
    ///
    /// 用于配置如何验证 `exp`, `nbf`, `iss`, `aud` 等标准声明。
    #[cfg(feature = "server-side")]
    validation: Validation,
}

/// ## 表示一个完整的 JWT，包含标准声明和自定义载荷。
///
/// 泛型参数 `P` 代表自定义的载荷 (Payload) 结构体。
#[derive(Serialize, Deserialize, Clone, Debug)]
#[serde(rename_all = "camelCase")]
pub struct Jwt<P> {
    /// (Issuer) 签发者
    pub iss: String,

    /// (Audience) 受众。可以是一个或多个。
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
    pub load: P,
}

/// ## JWT 令牌的载荷 (Payload) 中用于权限控制的部分。
#[derive(Serialize, Deserialize, Validate, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Permission {
    /// ## 允许的操作列表。
    ///
    /// 定义此令牌授权执行的具体 [`HTTP`](HttpMethod) 方法。
    pub methods: Vec<HttpMethod>,

    /// ## 资源路径模式。
    ///
    /// 定义此令牌可以访问的资源路径，支持通配符 `*` 和 `?` (Glob 模式)。
    ///
    /// 如果是 None，那么表示这个令牌没有任何对象的操作权限
    #[validate(length(max = 128))]
    pub resource_pattern: Option<String>,

    /// ## 允许上传的最大对象大小 (字节)。
    ///
    /// `None` 表示没有限制。
    pub max_size: Option<usize>,

    /// ## 允许的内容类型 (MIME types)。
    ///
    /// 支持通配符，例如 `image/*` 或 `*` (Glob 模式)。
    ///
    /// **大小有限制，每一个通配模式不超过 128 字节、最多 8 个模式**
    #[validate(custom(function = "Self::validate_content_type_pattern"))]
    pub allowed_content_types: Vec<String>,
}

#[cfg(feature = "server-side")]
pub struct CompiledPermission {
    pub methods: Vec<HttpMethod>,
    pub resource_pattern: Option<String>,
    pub max_size: Option<usize>,
    pub allowed_content_types: Vec<String>,
    resource_pattern_cache: Option<Pattern>,
    allowed_content_types_cache: Vec<Pattern>,
}

/// HTTP 操作方法枚举。
///
/// [`ValueEnum`] 用于 [`clap`] 集成，使其可以在命令行参数中使用。
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
    /// 代表所有安全的 HTTP 方法，你可以参看 [`HttpMethod::safe`] 获取 **安全** 一词的含义
    Safe,
    /// 代表所有不安全的 HTTP 方法，你可以参看 [`HttpMethod::safe`] 获取 **安全** 一词的含义
    Unsafe,
}

impl JwtEncoder {
    #[inline]
    pub fn new(encoding_key: HashMap<String, (EncodingKey, Algorithm)>) -> Self {
        let kids = encoding_key.keys().cloned().collect();
        Self { encoding_key, kids }
    }

    /// ## 将 JWT 声明编码为字符串形式的 Token
    ///
    /// **注意**：header 中的 alg 字段和 kid 对应的加密算法需要保持一致
    #[inline]
    pub fn encode<P: Serialize>(
        &self,
        claims: &Jwt<P>,
        kid: &str,
    ) -> Result<String, AuthError> {
        use AuthError::InternalError;

        let (key, alg) = self
            .encoding_key
            .get(kid)
            .ok_or(InternalError("No such kid found in your encoder".into()))?;

        let mut header = Header::new(*alg);
        header.kid = Some(kid.to_string());

        Ok(jsonwebtoken::encode(&header, claims, key)?)
    }

    pub fn encode_randomly<P: Serialize>(&self, claims: &Jwt<P>) -> Result<String, AuthError> {
        let random_kid = &self.kids[rand::random_range(..self.kids.len())];
        self.encode(claims, random_kid)
    }
}

#[cfg(feature = "server-side")]
impl JwtDecoder {
    /// ## 新建一个 [`JwtDecoder`]
    ///
    /// ### 参数说明
    ///
    /// - `mapping` `iss`、`kid` 到 [`DecodingKey`] 的映射，注意  [`mapping`](HashMap) 的联合主键的顺序是 (iss, kid)，别搞反了！
    /// - `algorithms`    接受的算法
    /// - `iss`     接受的令牌的签发人
    /// - `aud`     接受的令牌中的 aud 值
    ///
    /// ### panic
    ///
    /// - 如果 `algorithms` 中一个算法都没有，即 `algorithms` 是一个空的切片
    ///
    /// ### 新建完成后可以通过以下函数修改相应的配置
    ///
    /// - [`iss_kid_dec`](JwtDecoder::iss_kid_dec)
    /// - [`algorithms`](JwtDecoder::algorithms)
    /// - [`authorized_issuer`](JwtDecoder::authorized_issuer)
    /// - [`possible_audience`](JwtDecoder::possible_audience)
    /// - [`leeway`](JwtDecoder::leeway)
    /// - [`reject_tokens_expiring_in_less_than`](JwtDecoder::reject_tokens_expiring_in_less_than)
    ///
    /// ### 然后可以使用方法 [`decode`](JwtDecoder::decode) 来解码、校验一个 jwt
    ///
    pub fn new<T: ToString, U: ToString>(
        mapping: HashMap<(String, String), DecodingKey>,
        algorithms: &[Algorithm],
        iss: &[T],
        aud: &[U],
    ) -> Self {
        let mut validation =
            Validation::new(*algorithms.first().expect(
                "You should provide at least one algorithm in your accepted algorithm slice!",
            ));
        validation.validate_aud = true;
        validation.validate_exp = true;
        validation.validate_nbf = true;
        validation.algorithms = algorithms.to_vec();
        validation.reject_tokens_expiring_in_less_than = 0;
        validation.leeway = 60;
        validation.set_issuer(iss);
        validation.set_audience(aud);

        // 必须有下面的四个字段，否则视为非法 token，
        // jsonwebtoken 只接受下面的这些和 sub 字段，所以 iat 限制无法设置
        // 当然，如果没有，serde 也会自己产生反序列化错误，所以应该没问题……吧

        validation.set_required_spec_claims(&["aud", "exp", "nbf", "iss"]);
        Self {
            decoding_keys: mapping,
            validation,
        }
    }

    /// ## 设置 (iss, kid) 到 [`DecodingKey`] 的映射
    ///
    /// 注意  [`mapping`](HashMap) 的联合主键的顺序是 (iss, kid)，别搞反了！
    #[inline]
    pub fn iss_kid_dec(mut self, mapping: HashMap<(String, String), DecodingKey>) -> Self {
        self.decoding_keys = mapping;
        self
    }

    /// ## 设置接受的算法
    #[inline]
    pub fn algorithms(mut self, algorithms: &[Algorithm]) -> Self {
        self.validation.algorithms = algorithms.to_vec();
        self
    }

    /// ## 设置接受的 issuer
    #[inline]
    pub fn authorized_issuer<T: ToString>(mut self, iss: &[T]) -> Self {
        self.validation.set_issuer(iss);
        self
    }

    /// ## 设置接受的 audience
    #[inline]
    pub fn possible_audience<T: ToString>(mut self, aud: &[T]) -> Self {
        self.validation.set_audience(aud);
        self
    }

    /// ## 设置接受的 leeway
    #[inline]
    pub const fn leeway(mut self, leeway: u64) -> Self {
        self.validation.leeway = leeway;
        self
    }

    /// ## 临期的 token 不予通过
    #[inline]
    pub const fn reject_tokens_expiring_in_less_than(mut self, tolerance: u64) -> Self {
        self.validation.reject_tokens_expiring_in_less_than = tolerance;
        self
    }

    /// ## 使用给定的配置解码并验证一个字符串形式的 Token。
    ///
    /// 此函数会执行完整的验证流程，包括：
    /// 1. 检查签名是否有效。
    /// 2. 验证 `exp` 和 `nbf` 时间戳。
    /// 3. 根据 `config.validation` 中的设置验证 `iss` 和 `aud`。
    ///
    /// ### 泛型参数说明
    ///
    /// 注意这个函数的泛型参数 `P` 代表的是 **载荷 (Payload)** 的类型，而不是 `Jwt` 本身。
    ///
    /// ### 代码示例
    ///
    /// #### 推荐写法 (Best Practice)
    ///
    /// 利用 Rust 的类型推断，显式标注变量类型，代码最为清晰：
    ///
    /// ```rust,no_run
    /// # use crab_vault_auth::{JwtDecoder, Jwt, Permission, error::AuthError};
    /// # fn example(decoder: &JwtDecoder, token: &str) -> Result<(), AuthError> {
    /// // 编译器会自动推断出 P 是 Permission
    /// let jwt: Jwt<Permission> = decoder.decode(token)?;
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// #### 显式泛型写法
    ///
    /// 也可以使用 Turbofish 语法显式指定载荷类型：
    ///
    /// ```rust,no_run
    /// # use crab_vault_auth::{JwtDecoder, Jwt, Permission, error::AuthError};
    /// # fn example(decoder: &JwtDecoder, token: &str) -> Result<(), AuthError> {
    /// // 注意：尖括号内只需填 Permission
    /// let jwt = decoder.decode::<Permission>(token)?;
    /// // 此时 jwt 的类型为 Jwt<Permission>
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// #### 错误写法 (编译失败)
    ///
    /// 不要将 `Jwt<Permission>` 作为泛型参数传入，否则会导致类型嵌套 (`Jwt<Jwt<P>>`)，
    /// 这会导致类型不匹配从而**编译失败**：
    ///
    /// ```rust,compile_fail
    /// # use crab_vault_auth::{JwtDecoder, Jwt, Permission, AuthError};
    /// # fn example(decoder: &JwtDecoder, token: &str) -> Result<(), AuthError> {
    /// // 错误：decode 返回的是 Jwt<T>。
    /// // 如果传入 T = Jwt<Permission>，返回值就是 Jwt<Jwt<Permission>>。
    /// // 这与左侧的变量类型 Jwt<Permission> 不匹配。
    /// let jwt: Jwt<Permission> = decoder.decode::<Jwt<Permission>>(token)?;
    /// # Ok(())
    /// # }
    /// ```
    #[cfg(feature = "server-side")]
    pub fn decode<P>(&self, token: &str) -> Result<Jwt<P>, AuthError>
    where
        for<'de> P: Deserialize<'de>,
    {
        let kid = jsonwebtoken::decode_header(token)?
            .kid
            .ok_or(AuthError::MissingClaim("kid".to_string()))?;

        let body_unchecked: Jwt<P> = serde_json::from_value(Self::decode_unchecked(token)?)?;

        let key = self
            .decoding_keys
            .get(&(body_unchecked.iss, kid))
            .ok_or(AuthError::InvalidIssuer)?;

        Ok(jsonwebtoken::decode::<Jwt<P>>(token, key, &self.validation)?.claims)
    }

    /// ## **\[不安全\]** 在不验证签名的情况下解码 JWT 的载荷。
    ///
    /// # 警告
    ///
    /// **绝对不要**相信此函数返回的数据！因为它**没有验证** JWT 的签名。
    /// 这意味着任何人都可以伪造这个 JWT 的内容。
    ///
    /// 此函数仅应用于需要查看 Token 内容的调试或日志记录场景。
    /// 在任何与安全相关的逻辑中，都**必须**使用 [`JwtDecoder::decode`]。
    #[cfg(feature = "server-side")]
    pub fn decode_unchecked(token: &str) -> Result<serde_json::Value, AuthError> {
        let mut parts = token.split('.');
        let _header = parts.next();
        let payload = parts.next().ok_or(AuthError::InvalidToken)?;

        let decoded_payload = base64::engine::general_purpose::URL_SAFE_NO_PAD.decode(payload)?;
        let json_value = serde_json::from_slice(&decoded_payload)?;

        Ok(json_value)
    }
}

impl<P: Serialize + for<'de> Deserialize<'de>> Jwt<P> {
    /// 创建一个新的 `Jwt` 实例，并填入默认值。
    ///
    /// 默认值:
    /// - `iss`: `None`
    /// - `aud`: 空 `Vec`
    /// - `exp`: `一小时后` 的时间戳
    /// - `nbf`: `0` (立即生效)
    /// - `iat`: 当前时间的 Unix 时间戳
    /// - `jti`: 一个使用 [`Uuid::new_v4`] 新生成的 [`Uuid`]
    #[inline]
    pub fn new<T: ToString, U: ToString>(iss: T, aud: &[U], payload: P) -> Self {
        let now = chrono::Utc::now().timestamp();
        Self {
            iss: iss.to_string(),
            aud: aud.iter().map(|s| s.to_string()).collect(),
            exp: now + 3600,
            nbf: now,
            iat: now,
            jti: Uuid::new_v4(),
            load: payload,
        }
    }

    /// 设置 JWT 的相对过期时间，从现在开始计算。
    #[inline]
    pub fn expires_in(mut self, duration: chrono::Duration) -> Self {
        self.exp = (chrono::Utc::now() + duration).timestamp();
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

    /// !!! 永不过期 !!!
    #[inline]
    pub const fn never_expires(mut self) -> Self {
        self.exp = i32::MAX as i64;
        self
    }

    /// 设置 JWT 的生效时间，从现在开始计算。
    #[inline]
    pub fn not_valid_in(mut self, duration: chrono::Duration) -> Self {
        self.nbf = (chrono::Utc::now() + duration).timestamp();
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
    pub const fn uuid(mut self, id: Uuid) -> Self {
        self.jti = id;
        self
    }
}

impl Default for Permission {
    #[inline]
    fn default() -> Self {
        Self::new_minimum()
    }
}

impl Permission {
    fn validate_content_type_pattern(patterns: &[String]) -> Result<(), ValidationError> {
        if patterns.len() <= 8 && patterns.iter().all(|s| s.len() <= 128) {
            Ok(())
        } else {
            Err(ValidationError::new("pattern too long/much for parsing"))
        }
    }

    #[inline]
    pub const fn new() -> Self {
        Self::new_minimum()
    }

    /// 创建一个 <u>**拥有所有权限**</u> 的 `root` `Permission`。
    ///
    /// ### 这个操作应当尽量少用，因为这个获取这个权限就意味着该用户能够读写所有的资源 (所有！)
    ///
    /// 默认值
    ///
    /// - 允许操作: [`HttpMethod::All`]
    /// - 允许资源: [`Some("*".to_string())`](Some) (所有路径)
    /// - 大小限制：[`None`]
    /// - MIME: **所有**
    pub fn new_root() -> Self {
        Self {
            methods: vec![HttpMethod::All],
            resource_pattern: Some("*".to_string()),
            max_size: None,
            allowed_content_types: vec!["*".to_string()],
        }
    }

    /// 创建一个 <u>**没有任何权限**</u> 的 "minimum" `Permission`。
    ///
    /// 直接签发这个 [`Permission`] 将导致完全无法访问任何内容
    ///
    /// 默认值
    ///
    /// - 允许操作: 无（一个空的 vec）
    /// - 允许资源: [`None`] (所有路径都不允许)
    /// - 大小限制：[`Some(0)`](Some) (上传的最大包大小为 0 字节)
    /// - MIME: **所有都不行**
    pub const fn new_minimum() -> Self {
        Self {
            methods: vec![],
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
        self.methods = methods;
        self
    }

    /// 修改这个令牌能够访问的资源路径
    #[inline]
    pub fn permit_resource_pattern<T>(mut self, pattern: T) -> Self
    where
        T: Into<String>,
    {
        self.resource_pattern = Some(pattern.into());
        self
    }

    /// 修改这个令牌能够访问的资源路径
    #[inline]
    pub fn permit_resource_pattern_option<T>(mut self, pattern: Option<T>) -> Self
    where
        T: Into<String>,
    {
        self.resource_pattern = pattern.map(T::into);
        self
    }

    /// 设置最大的内容长度
    #[inline]
    pub const fn restrict_maximum_size(mut self, max: usize) -> Self {
        self.max_size = Some(max);
        self
    }

    #[inline]
    pub const fn restrict_maximum_size_option(mut self, max: Option<usize>) -> Self {
        self.max_size = max;
        self
    }

    /// 此令牌允许的最大内容类型
    #[inline]
    pub fn permit_content_type(mut self, content_type: Vec<String>) -> Self {
        self.allowed_content_types = content_type;
        self
    }

    #[cfg(feature = "server-side")]
    pub fn compile(self) -> CompiledPermission {
        let Permission {
            methods,
            resource_pattern,
            max_size,
            allowed_content_types,
        } = self;

        let resource_pattern_cache = match &resource_pattern {
            Some(pat) => Pattern::new(pat).ok(),
            None => None,
        };

        let mut allowed_content_types_cache = vec![];

        for pat in &allowed_content_types {
            if let Ok(pat) = Pattern::new(pat) {
                allowed_content_types_cache.push(pat)
            }
        }

        CompiledPermission {
            methods,
            resource_pattern,
            max_size,
            allowed_content_types,
            resource_pattern_cache,
            allowed_content_types_cache,
        }
    }
}

#[cfg(feature = "server-side")]
impl CompiledPermission {
    /// ## 检查此权限是否允许执行给定的 HTTP 方法。
    ///
    /// 此方法会依次检查：
    ///
    /// 1. [`Permission`] 中含有 [`All`](HttpMethod::All)，返回 `true`
    /// 2. [`Permission`] 中含有提供的 [`method`](HttpMethod)，返回 `true`
    /// 3. [`Permission`] 中是否含有 [`Safe`](HttpMethod::Safe)，若有，且提供的 [`method`](HttpMethod) 的确是安全的，返回 `true`
    /// 4. [`Permission`] 中是否含有 [`Unsafe`](HttpMethod::Unsafe)，若有，且提供的 [`method`](HttpMethod) 的确是不安全的，返回 `true`
    /// 5. 其他，返回 false
    pub fn can_perform_method(&self, method: HttpMethod) -> bool {
        self.methods.contains(&HttpMethod::All)
            || self.methods.contains(&method)
            || (self.methods.contains(&HttpMethod::Safe) && method.safe())
            || (self.methods.contains(&HttpMethod::Unsafe) && !method.safe())
    }

    /// ## 检查此权限是否能访问给定的资源路径。
    ///
    /// 使用 `resource_pattern` 对 `path` 进行 Glob 匹配。
    ///
    /// - 如果 `resource_pattern` 不是一个有效的 Glob 模式，会安全地返回 `false`。
    /// - 如果是一个 [`None`] 也会返回 false，因为规定了 [`None`] 表示所有都不能访问
    pub fn can_access(&self, path: &str) -> bool {
        match &self.resource_pattern_cache {
            Some(pat) => pat.matches(path),
            None => false,
        }
    }

    /// ## 检查给定的大小是否在 `max_size` 的限制内。
    ///
    /// - 如果 `max_size` 是 `None` (无限制)
    /// - 或者 `size` 小于等于限制，则返回 `true`。
    pub fn check_size(&self, size: usize) -> bool {
        self.max_size.is_none_or(|limit| size <= limit)
    }

    /// ## 检查给定的内容类型是否被允许。
    ///
    /// 遍历 `allowed_content_types`，对每个模式进行 Glob 匹配。
    pub fn check_content_type(&self, content_type: &str) -> bool {
        self.allowed_content_types_cache
            .iter()
            .any(|allow_pat| allow_pat.matches(content_type))
    }
}

impl From<&axum::http::Method> for HttpMethod {
    fn from(value: &axum::http::Method) -> Self {
        use axum::http::Method;

        match *value {
            Method::GET => Self::Get,
            Method::POST => Self::Post,
            Method::PUT => Self::Put,
            Method::PATCH => Self::Patch,
            Method::DELETE => Self::Delete,
            Method::HEAD => Self::Head,
            Method::OPTIONS => Self::Options,
            Method::TRACE => Self::Trace,
            Method::CONNECT => Self::Connect,
            _ => Self::Other,
        }
    }
}

impl From<axum::http::Method> for HttpMethod {
    fn from(value: axum::http::Method) -> Self {
        Self::from(&value)
    }
}

impl HttpMethod {
    /// ## 判断一个方法是否安全
    ///
    /// 根据 [MDN](https://developer.mozilla.org/zh-CN/docs/Glossary/Safe/HTTP)
    /// 以及 [rfc7231](https://datatracker.ietf.org/doc/html/rfc7231#section-4.2.1)
    /// 对于安全的定义
    ///
    /// 一个方法是否安全取决于该方法的请求在被服务器响应后，<u>**服务器的状态是否改变**</u>
    ///
    /// 或者说一个方法安不安全取决于是否蕴含着**写入请求**
    ///
    /// 所以，对于 [`OPTIONS`](HttpMethod::Options) 这类在通常认知中
    /// 会造成服务器信息暴露等问题的方法，仍然认为是安全的
    ///
    ///
    /// - 如果一个方法是只读的，如 [`HEAD`](HttpMethod::Head)，[`GET`](HttpMethod::Get) 等，那他就是安全的
    /// - 如果一个方法有写入的含义，如 [`PUT`](HttpMethod::Put)，[`DELETE`](HttpMethod::Delete) 等，那么就不安全
    ///
    /// 同时，在这里，由于有两个例外：[`HttpMethod::Other`] 和 [`HttpMethod::All`] 这两个标记
    ///
    /// 它们两个一个代表其他请求（rfc规范之外的），一个代表所有的请求，包括 rfc 规范之外的，所以都视为不安全
    pub fn safe(self) -> bool {
        match self {
            // safe 不必说，必然是安全的
            HttpMethod::Safe
            | HttpMethod::Get
            | HttpMethod::Head
            | HttpMethod::Options
            | HttpMethod::Trace => true,
            // unsafe operations，这些操作会导致内容改变
            HttpMethod::Unsafe
            | HttpMethod::Connect
            | HttpMethod::Post
            | HttpMethod::Put
            | HttpMethod::Patch
            | HttpMethod::Delete
            | HttpMethod::Other
            | HttpMethod::All => false,
        }
    }
}
