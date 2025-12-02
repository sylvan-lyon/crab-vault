use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase", untagged)]
pub enum ApiError {
    Client(ClientError),
    Server(ServerError),
}

#[non_exhaustive]
#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum ClientError {
    /// 没有 content type 这个头部
    MissingContentType,

    /// content type 这个头部的值没有通过 [`Permission`](crab_vault::auth::Permission) 校验
    InvalidContentType,

    /// 没有 content length 这个头部
    MissingContentLength,

    /// 报文部分太大了
    BodyTooLarge,

    /// uri 错误
    UriInvalid,

    /// 值解析出错，比如 content length 应该是一个数字，但是你传了一个字符串
    ValueParsingError,

    /// HTTP 规定头部中不允许有除了 **可见** ASCII 之外的字符，如果有，就会产生这个错误
    HeaderWithOpaqueBytes,

    /// base64 解码错误
    Base64DecodeError,

    JsonError {
        kind: &'static str,
        line: usize,
        col: usize,
    },
}

#[non_exhaustive]
#[derive(Serialize)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum ServerError {
    Internal,
}

impl ClientError {
    pub fn code(&self) -> StatusCode {
        match self {
            ClientError::MissingContentType
            | ClientError::InvalidContentType
            | ClientError::MissingContentLength
            | ClientError::BodyTooLarge
            | ClientError::HeaderWithOpaqueBytes
            | ClientError::Base64DecodeError
            | ClientError::ValueParsingError
            | ClientError::JsonError {
                kind: _,
                col: _,
                line: _,
            } => StatusCode::UNPROCESSABLE_ENTITY,

            ClientError::UriInvalid => StatusCode::NOT_FOUND,
        }
    }
}

impl ServerError {
    pub fn code(&self) -> StatusCode {
        StatusCode::NOT_FOUND
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        match self {
            ApiError::Client(e) => (e.code(), axum::Json(e)).into_response(),
            ApiError::Server(e) => (e.code(), axum::Json(e)).into_response(),
        }
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
        Self::Client(ClientError::BodyTooLarge)
    }
}

impl From<axum::http::header::ToStrError> for ApiError {
    fn from(_: axum::http::header::ToStrError) -> Self {
        Self::Client(ClientError::HeaderWithOpaqueBytes)
    }
}

impl From<base64::DecodeError> for ApiError {
    fn from(_: base64::DecodeError) -> Self {
        Self::Client(ClientError::Base64DecodeError)
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        let kind = match e.classify() {
            serde_json::error::Category::Io => "io",
            serde_json::error::Category::Syntax => "syntax",
            serde_json::error::Category::Data => "data",
            serde_json::error::Category::Eof => "eof",
        };

        let (line, col) = (e.line(), e.column());

        Self::Client(ClientError::JsonError { kind, line, col })
    }
}
