use axum::{
    Router, debug_handler,
    extract::{Path, State},
    http::{StatusCode, header::CONTENT_LENGTH},
    response::IntoResponse,
};

use crate::{api::AppState, common::errors::StorageError, storage::DataStorage};

pub(super) fn build_router() -> Router<AppState> {
    use axum::routing::MethodRouter;

    let method_router = MethodRouter::new()
        .put(upload_object)
        .get(get_object)
        .head(head_object)
        .delete(delete_object);

    Router::new().route("/{bucket_name}/{*object_name}", method_router)
}

#[debug_handler]
async fn upload_object(
    State(state): State<AppState>,
    Path((bucket, object)): Path<(String, String)>,
    body: bytes::Bytes,
) -> Result<impl IntoResponse, StorageError> {
    state
        .data_src
        .create_object(&bucket, &object, &body)
        .await?;
    Ok(StatusCode::CREATED)
}

#[debug_handler]
async fn get_object(
    State(state): State<AppState>,
    Path((bucket, object)): Path<(String, String)>,
) -> Result<impl IntoResponse, StorageError> {
    let data = state.data_src.read_object(&bucket, &object).await?;
    Ok(data)
}

#[debug_handler]
async fn delete_object(
    State(state): State<AppState>,
    Path((bucket, object)): Path<(String, String)>,
) -> Result<impl IntoResponse, StorageError> {
    state.data_src.delete_object(&bucket, &object).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[debug_handler]
async fn head_object(
    State(state): State<AppState>,
    Path((bucket, object)): Path<(String, String)>,
) -> Result<impl IntoResponse, StorageError> {
    let len = state.data_src.head_object(&bucket, &object).await?;
    Ok((StatusCode::OK, [(CONTENT_LENGTH, len)]))
}
