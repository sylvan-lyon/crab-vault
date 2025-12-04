use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::Serialize;
use thiserror::Error;

pub type EngineResult<T> = Result<T, EngineError>;

#[derive(Debug, Serialize, Error)]
#[serde(rename_all = "camelCase", tag = "code")]
pub enum EngineError {
    #[error("io error: {error} while manipulating {path}")]
    Io {
        #[serde(skip)]
        error: std::io::Error,
        path: String,
    },

    #[error("ser/de error: {error} at line{line}, column {column}")]
    Serde {
        error: &'static str,
        line: usize,
        column: usize,
    },

    #[error("bucket not found: {bucket}")]
    BucketNotFound { bucket: String },

    #[error("bucket meta not found: {bucket}")]
    BucketMetaNotFound { bucket: String },

    #[error("bucket not empty, possibly while deleting, details {bucket}")]
    BucketNotEmpty { bucket: String },

    #[error("object not found: {bucket}/{object}")]
    ObjectNotFound { bucket: String, object: String },

    #[error("object meta not found: {bucket}/{object}")]
    ObjectMetaNotFound { bucket: String, object: String },

    #[allow(dead_code)]
    #[error("some other errors: {0}")]
    Other(String),

    #[allow(dead_code)]
    #[error("backend error: {0}")]
    BackendError(String),

    #[error("invalid argument: {0}")]
    InvalidArgument(String),
}

impl From<serde_json::error::Error> for EngineError {
    fn from(value: serde_json::error::Error) -> Self {
        use serde_json::error::Category;
        let kind = match value.classify() {
            Category::Io => "io",
            Category::Syntax => "syntax",
            Category::Data => "data",
            Category::Eof => "eof",
        };

        EngineError::Serde {
            error: kind,
            line: value.line(),
            column: value.column(),
        }
    }
}

impl IntoResponse for EngineError {
    fn into_response(self) -> Response {
        use EngineError::*;
        let code = match &self {
            Serde {
                error: _,
                line: _,
                column: _,
            }
            | Io { error: _, path: _ }
            | BackendError(_)
            | Other(_) => StatusCode::INTERNAL_SERVER_ERROR,

            ObjectNotFound {
                bucket: _,
                object: _,
            }
            | BucketNotFound { bucket: _ } => StatusCode::NOT_FOUND,

            ObjectMetaNotFound {
                bucket: _,
                object: _,
            }
            | BucketMetaNotFound { bucket: _ } => StatusCode::NOT_FOUND,

            BucketNotEmpty { bucket: _ } => StatusCode::CONFLICT,
            InvalidArgument(_) => StatusCode::UNPROCESSABLE_ENTITY,
        };

        #[derive(Serialize)]
        struct Msg {
            #[serde(flatten)]
            error: EngineError,
            msg: String,
        }

        (
            code,
            axum::Json(Msg {
                msg: self.to_string(),
                error: self,
            }),
        )
            .into_response()
    }
}

impl From<EngineError> for Response {
    fn from(value: EngineError) -> Self {
        value.into_response()
    }
}
