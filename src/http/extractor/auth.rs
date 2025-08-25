use axum::{extract::FromRequestParts, http::request::Parts};

use crate::{error::auth::AuthError, http::auth::Permission};

/// 从请求扩展中提取权限信息的提取器
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