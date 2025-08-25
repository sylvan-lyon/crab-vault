use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Serialize, Clone, Error)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum AuthError {
    #[error("missing authorization header")]
    MissingAuthHeader,

    #[error("invalid authorization format: expected 'Bearer <token>'")]
    InvalidAuthFormat,

    #[error("jwt error: token is invalid")]
    TokenInvalid,

    #[error("jwt error: token has expired")]
    TokenExpired,

    #[error("jwt error: token is not yet valid")]
    TokenNotYetValid,

    #[error("jwt error: invalid signature")]
    InvalidSignature,

    #[error("jwt error: invalid issuer")]
    InvalidIssuer,

    #[error("jwt error: invalid audience")]
    InvalidAudience,

    #[error("jwt error: invalid subject")]
    InvalidSubject,

    #[error("jwt error: required claim missing: {0}")]
    MissingClaim(String),

    #[error("jwt error: insufficient permissions for this operation")]
    InsufficientPermissions,

    #[error("jwt error: token has been revoked")]
    TokenRevoked,

    #[allow(dead_code)]
    #[error("internal server error during authentication")]
    InternalError,
}

impl From<jsonwebtoken::errors::Error> for AuthError {
    fn from(value: jsonwebtoken::errors::Error) -> Self {
        use jsonwebtoken::errors::ErrorKind::*;

        match value.into_kind() {
            ExpiredSignature => AuthError::TokenExpired,
            InvalidSignature => AuthError::InvalidSignature,
            InvalidIssuer => AuthError::InvalidIssuer,
            InvalidAudience => AuthError::InvalidAudience,
            InvalidSubject => AuthError::InvalidSubject,
            MissingRequiredClaim(claim) => AuthError::MissingClaim(claim),
            _ => AuthError::TokenInvalid,
        }
    }
}

impl IntoResponse for AuthError {
    fn into_response(self) -> Response {
        let status_code = match self {
            AuthError::MissingAuthHeader
            | AuthError::InvalidAuthFormat
            | AuthError::TokenInvalid
            | AuthError::TokenExpired
            | AuthError::TokenNotYetValid
            | AuthError::InvalidSignature
            | AuthError::InvalidIssuer
            | AuthError::InvalidAudience
            | AuthError::InvalidSubject
            | AuthError::MissingClaim(_)
            | AuthError::TokenRevoked => StatusCode::UNAUTHORIZED,

            AuthError::InsufficientPermissions => StatusCode::FORBIDDEN,

            AuthError::InternalError => StatusCode::INTERNAL_SERVER_ERROR,
        };

        (status_code, axum::Json(self)).into_response()
    }
}

impl From<AuthError> for Response {
    #[inline(always)]
    fn from(val: AuthError) -> axum::response::Response {
        val.into_response()
    }
}
