use axum::{
    extract::{FromRequest, FromRequestParts, Request},
    http::request::Parts,
    response::{IntoResponse, Response},
};
use bytes::Bytes;

use crate::{
    error::{api::ApiError, auth::AuthError},
    http::auth::Permission,
};

#[allow(dead_code)]
pub struct PermissionExtractor(pub Permission);

impl<S> FromRequestParts<S> for PermissionExtractor
where
    S: Send + Sync,
{
    type Rejection = AuthError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Permission>()
            .cloned()
            .map(PermissionExtractor)
            .ok_or(AuthError::TokenInvalid)
    }
}

pub struct RestrictedBytes(pub Bytes);

impl<S> FromRequest<S> for RestrictedBytes
where
    S: Send + Sync,
{
    type Rejection = Response; // 发生错误时直接返回 Response

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let permission = match req.extensions().get::<Permission>() {
            Some(p) => p,
            // 如果没有找到权限，这是一个服务器内部错误。
            // 意味着这个提取器被用在了没有被 AuthMiddleware 保护的路由上。
            // 因为前面的 AuthMiddleware 被设计为如果设置某一个 API 不受保护，那么将返回一个最高级别的 Permission
            None => unreachable!(),
        }
        .clone();

        let body_bytes = match Bytes::from_request(req, state).await {
            Ok(bytes) => bytes,
            Err(e) => {
                let api_error: ApiError = e.into();
                return Err(api_error.into_response());
            }
        };

        if !permission.check_size(body_bytes.len() as u64) {
            return Err(ApiError::BodyTooLarge.into_response());
        }

        // 步骤 4: 验证通过，返回包装后的 Bytes
        Ok(RestrictedBytes(body_bytes))
    }
}
