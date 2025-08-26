use std::{
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
};

use axum::{
    http::{
        HeaderMap,
        header::{AUTHORIZATION, CONTENT_LENGTH, CONTENT_TYPE},
    },
    response::{IntoResponse, Response},
};
use tower::{Layer, Service};

use crate::{
    app_config::server::JwtConfig,
    error::{api::ApiError, auth::AuthError},
    http::auth::{HttpMethod, Jwt, Permission},
};

#[derive(Clone)]
pub struct AuthMiddleware<Inner> {
    inner: Inner,
    jwt_config: Arc<JwtConfig>,
}

// 在 Inner 是一个 Service 的情况下，可以为 AuthMiddleware<Inner> 实现 Service
// 这个 AuthMiddleware 和 Inner 使用同样的请求参数，axum::http::Request<ReqBody>
impl<Inner, ReqBody> Service<axum::http::Request<ReqBody>> for AuthMiddleware<Inner>
where
    Inner: Service<axum::http::Request<ReqBody>> + Send + Clone + 'static,
    ReqBody: 'static + Send,
    Inner::Error: std::error::Error,
    Inner::Response: IntoResponse,
    Inner::Future: 'static + Send,
{
    type Response = Response;
    type Error = Infallible;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx).map_err(|_| unreachable!())
    }

    fn call(&mut self, mut req: axum::http::Request<ReqBody>) -> Self::Future {
        let cloned = self.inner.clone();
        let mut inner = std::mem::replace(&mut self.inner, cloned);
        let jwt_config = self.jwt_config.clone();

        Box::pin(async move {
            match extract_and_validate_token(
                req.headers(),
                req.method().into(),
                req.uri().path(),
                &jwt_config,
            )
            .await
            {
                Ok(permission) => {
                    req.extensions_mut().insert(permission);
                    match inner.call(req).await {
                        Ok(val) => Ok(val.into_response()),
                        Err(_) => unreachable!(),
                    }
                }
                Err(e) => Ok(e),
            }
        })
    }
}

#[derive(Clone)]
pub struct AuthLayer(Arc<JwtConfig>);

impl AuthLayer {
    /// 此函数将在堆上创建一个 [`JwtConfig`] 结构作为这个中间件的配置
    pub fn new(config: Arc<JwtConfig>) -> Self {
        Self(config.clone())
    }
}

impl<Inner> Layer<Inner> for AuthLayer {
    type Service = AuthMiddleware<Inner>;

    fn layer(&self, service: Inner) -> Self::Service {
        AuthMiddleware {
            inner: service,
            jwt_config: self.0.clone(),
        }
    }
}

/// 提取并验证JWT令牌
async fn extract_and_validate_token(
    headers: &HeaderMap,
    method: HttpMethod,
    path: &str,
    jwt_config: &JwtConfig,
) -> Result<Permission, Response> {
    // 1. 提取Authorization头
    let auth_header = headers
        .get(AUTHORIZATION)
        .ok_or(AuthError::MissingAuthHeader)?
        .to_str()
        .map_err(|_| AuthError::InvalidAuthFormat)?;

    // 2. 验证Bearer格式并提取令牌
    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(AuthError::InvalidAuthFormat)?;

    // 3. 解码并验证JWT
    let jwt: Jwt<Permission> = Jwt::decode(token, jwt_config)?;

    // 4. 检查 content-length，如果没过这个要求，那更是演都不演了
    let content_length = headers
        .get(CONTENT_LENGTH)
        .ok_or(ApiError::MissingContentLength)?
        .to_str()
        .map_err(|_| ApiError::EncodingError)?
        .parse()
        .map_err(|_| ApiError::ValueParsingError)?;
    if !jwt.payload.check_size(content_length) {
        return Err(ApiError::BodyTooLarge.into());
    }

    // 5. 检查资源路径匹配和请求方法
    if !jwt.payload.can_perform(method) || !jwt.payload.can_access(path) {
        return Err(AuthError::InsufficientPermissions.into());
    }

    // 6. 检查 content-type
    let content_type = headers
        .get(CONTENT_TYPE)
        .ok_or(ApiError::MissingContentType)?
        .to_str()
        .map_err(|_| ApiError::InvalidContentType)?;
    if !jwt.payload.check_content_type(content_type) {
        return Err(ApiError::InvalidContentType.into());
    }

    Ok(jwt.payload)
}
