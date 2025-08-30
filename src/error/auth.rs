use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use jsonwebtoken::Algorithm;
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Serialize, Clone, Error)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum AuthError {
    #[error("missing authorization header")]
    MissingAuthHeader,

    #[error("algorithm `{:?}` unsupported", 0)]
    InvalidAlgorithm(Algorithm),

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

    #[error("jwt error: untrusted issuer")]
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

    #[error("internal server error during authentication, details: {0}")]
    InternalError(#[serde(skip)] String),
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
            ImmatureSignature => AuthError::TokenNotYetValid,
            InvalidToken => AuthError::TokenInvalid,

            InvalidEcdsaKey => AuthError::InternalError("the secret given is not a valid ECDSA key".into()),
            InvalidRsaKey(_) => AuthError::InternalError("the secret given is not a valid RSA key".into()),
            RsaFailedSigning => AuthError::InternalError("could not sign with the given key".into()),
            InvalidAlgorithmName => AuthError::InternalError("cannot parse algorithm from str".into()),
            InvalidKeyFormat => AuthError::InternalError("a key is provided with an invalid format".into()),
            InvalidAlgorithm => AuthError::InternalError("the algorithm in the header doesn't match the one passed to decode or the encoding/decoding key used doesn't match the alg requested".to_string()),
            MissingAlgorithm => AuthError::InternalError("the Validation struct does not contain at least 1 algorithm".into()),
            Base64(e) => AuthError::InternalError(format!("An error happened when decoding some base64 text: {e}")),
            Json(e) => AuthError::InternalError(format!("An error happened while serializing/deserializing JSON: {e}")),
            Utf8(e) => AuthError::InternalError(format!("Some of the text was invalid UTF-8: {e}")),
            Crypto(e) => AuthError::InternalError(format!("Something unspecified went wrong with crypto: {e}")),
            _ => todo!()
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
            | AuthError::InvalidAlgorithm(_)
            | AuthError::InvalidSignature
            | AuthError::InvalidIssuer
            | AuthError::InvalidAudience
            | AuthError::InvalidSubject
            | AuthError::MissingClaim(_)
            | AuthError::TokenRevoked => StatusCode::UNAUTHORIZED,

            AuthError::InsufficientPermissions => StatusCode::FORBIDDEN,

            AuthError::InternalError(_) => StatusCode::INTERNAL_SERVER_ERROR,
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
