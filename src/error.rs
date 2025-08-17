use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::io;

#[derive(Debug)]
pub enum StorageError {
    Io(io::Error),
    NotFound,
}

impl From<io::Error> for StorageError {
    fn from(err: io::Error) -> Self {
        StorageError::Io(err)
    }
}

impl IntoResponse for StorageError {
    fn into_response(self) -> Response {
        match self {
            StorageError::Io(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
            StorageError::NotFound => (StatusCode::NOT_FOUND, "Object not found").into_response(),
        }
    }
}