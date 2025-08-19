use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::{fmt::Display, io};

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error, String),
    Serialization(serde_json::Error),
    Deserialization(serde_json::Error),
    #[allow(dead_code)]
    BackendError(String),

    #[allow(dead_code)]
    BucketNotFound(String),
    #[allow(dead_code)]
    ObjectNotFound(String, String),

    #[allow(dead_code)]
    BucketAlreadyExists(String),
    #[allow(dead_code)]
    ObjectAlreadyExists(String, String),

    #[allow(dead_code)]
    InvalidArgument(String),
}

impl Display for StorageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use StorageError::*;
        match self {
            Io(e, path) => f.write_fmt(format_args!("io error: {e} while manipulating {path}")),
            Serialization(e) => f.write_fmt(format_args!("serialize error: {e}")),
            Deserialization(e) => f.write_fmt(format_args!("deserialize error: {e}")),
            BackendError(e) => f.write_fmt(format_args!("backend error: {e}")),
            BucketNotFound(e) => f.write_fmt(format_args!("bucket not found: {e}")),
            ObjectNotFound(b, o) => f.write_fmt(format_args!("object not found: {b}/{o}")),
            BucketAlreadyExists(e) => f.write_fmt(format_args!("bucket already exists: {e}")),
            ObjectAlreadyExists(b, o) => {
                f.write_fmt(format_args!("object already exists: {b}/{o}"))
            }
            InvalidArgument(e) => f.write_fmt(format_args!("invalid argument: {e}")),
        }
    }
}

impl core::error::Error for StorageError {}

impl From<io::Error> for StorageError {
    fn from(err: io::Error) -> Self {
        StorageError::Io(err, "Converted from io::Error".to_string())
    }
}

impl IntoResponse for StorageError {
    fn into_response(self) -> Response {
        use StorageError::*;
        let code = match &self {
            Io(_, _) | Serialization(_) | Deserialization(_) | BackendError(_) => {
                StatusCode::INTERNAL_SERVER_ERROR
            }
            BucketNotFound(_) | ObjectNotFound(_, _) => StatusCode::NOT_FOUND,
            BucketAlreadyExists(_) | ObjectAlreadyExists(_, _) => StatusCode::OK,
            InvalidArgument(_) => StatusCode::UNPROCESSABLE_ENTITY,
        };

        let msg = self.to_string();

        tracing::error!("{self}");

        (code, msg).into_response()
    }
}
