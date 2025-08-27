use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum ApiError {
    // 客户端错误
    MissingContentType,
    InvalidContentType,

    MissingContentLength,
    BodyTooLarge,

    UriInvalid,

    EncodingError,
    ValueParsingError,
    // 服务器错误
}

impl ApiError {
    pub fn code(&self) -> StatusCode {
        match self {
            ApiError::MissingContentType
            | ApiError::InvalidContentType
            | ApiError::MissingContentLength
            | ApiError::BodyTooLarge
            | ApiError::EncodingError
            | ApiError::ValueParsingError => StatusCode::UNPROCESSABLE_ENTITY,

            ApiError::UriInvalid => StatusCode::NOT_FOUND,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        (self.code(), axum::Json(self)).into_response()
    }
}

impl From<ApiError> for Response {
    #[inline(always)]
    fn from(value: ApiError) -> Self {
        value.into_response()
    }
}

impl From<axum::extract::rejection::BytesRejection> for ApiError {
    fn from(_: axum::extract::rejection::BytesRejection) -> Self {
        Self::BodyTooLarge
    }
}
