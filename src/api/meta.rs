use axum::{
    Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{self, MethodRouter},
};

use crate::{
    api::AppState,
    common::errors::StorageError,
    storage::{BucketMeta, MetaStorage, ObjectMeta},
};

pub(super) fn build_router() -> Router<AppState> {
    let bucket_method_router = MethodRouter::new()
        .put(put_bucket_meta)
        .get(get_bucket_meta)
        .delete(delete_bucket_meta);

    let object_method_router = MethodRouter::new()
        .put(put_object_meta)
        .get(get_object_meta)
        .delete(delete_object_meta);

    Router::new()
        // buckets
        .route("/bucket/", routing::get(list_buckets_meta))
        .route("/bucket/{bucket_name}", bucket_method_router)
        // objects
        .route("/object/{bucket_name}", routing::get(list_objects_meta))
        .route("/object/{bucket_name}/{*object_name}", object_method_router)
}

async fn list_buckets_meta(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, StorageError> {
    let meta = state.meta_src.list_buckets_meta().await?;
    Ok((StatusCode::OK, axum::Json(meta)))
}

async fn put_bucket_meta(
    State(state): State<AppState>,
    Path(name): Path<String>,
    Json(meta): Json<BucketMeta>,
) -> Result<impl IntoResponse, StorageError> {
    let meta = BucketMeta { name, ..meta };
    state.meta_src.put_bucket_meta(&meta).await
}

async fn get_bucket_meta(
    State(state): State<AppState>,
    Path(bucket_name): Path<String>,
) -> Result<impl IntoResponse, StorageError> {
    let meta = state.meta_src.get_bucket_meta(&bucket_name).await?;
    Ok((StatusCode::OK, axum::Json(meta)))
}

async fn delete_bucket_meta(
    State(state): State<AppState>,
    Path(bucket_name): Path<String>,
) -> Result<impl IntoResponse, StorageError> {
    state.meta_src.delete_bucket_meta(&bucket_name).await
}

async fn put_object_meta(
    State(state): State<AppState>,
    Path((bucket_name, object_name)): Path<(String, String)>,
    Json(meta): Json<ObjectMeta>,
) -> Result<impl IntoResponse, StorageError> {
    let meta = ObjectMeta {
        object_name,
        bucket_name,
        ..meta
    };
    state.meta_src.put_object_meta(&meta).await
}

async fn get_object_meta(
    State(state): State<AppState>,
    Path((bucket_name, object_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, StorageError> {
    let meta = state
        .meta_src
        .get_object_meta(&bucket_name, &object_name)
        .await?;
    Ok((StatusCode::OK, axum::Json(meta)))
}

async fn delete_object_meta(
    State(state): State<AppState>,
    Path((bucket_name, object_name)): Path<(String, String)>,
) -> Result<impl IntoResponse, StorageError> {
    state
        .meta_src
        .delete_object_meta(&bucket_name, &object_name)
        .await
}

async fn list_objects_meta(
    State(state): State<AppState>,
    Path(bucket_name): Path<String>,
) -> Result<impl IntoResponse, StorageError> {
    let meta = state.meta_src.list_objects_meta(&bucket_name).await?;
    Ok((StatusCode::OK, axum::Json(meta)))
}
