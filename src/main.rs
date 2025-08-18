mod app_config;
mod common;
mod logger;
mod storage;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put},
};
use std::{net::Ipv4Addr, sync::Arc, time::Duration};
use storage::FileStorage;
use tower_http::cors::{self, CorsLayer};

use crate::common::errors::*;

type AppState = Arc<FileStorage>;

#[tokio::main]
async fn main() {
    let conf_ref = &app_config::CONFIG;
    logger::init();

    let storage = FileStorage::new(conf_ref.data_mnt_point()).expect("Failed to create storage");
    let state = Arc::new(storage);

    let cors_layer = CorsLayer::new()
        .allow_credentials(false)
        .allow_headers(cors::Any)
        .allow_methods(cors::Any)
        .allow_origin(cors::Any)
        .max_age(Duration::from_secs(3600 * 12));

    let app = Router::new()
        .route("/{bucket}/{object}", put(upload_object))
        .route("/{bucket}/{object}", get(get_object))
        .route("/{bucket}/{object}", delete(delete_object))
        .route_layer(cors_layer)
        .with_state(state);

    let listener =
        tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, app_config::CONFIG.port()))
            .await
            .unwrap();

    tracing::info!(
        "Server running on http://{}",
        listener.local_addr().unwrap()
    );

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}

// PUT - 上传对象
async fn upload_object(
    State(storage): State<AppState>,
    Path((bucket, object)): Path<(String, String)>,
    body: bytes::Bytes,
) -> Result<impl IntoResponse, StorageError> {
    storage.put_object(&bucket, &object, &body).await?;
    Ok(StatusCode::CREATED)
}

// GET - 获取对象
async fn get_object(
    State(storage): State<AppState>,
    Path((bucket, object)): Path<(String, String)>,
) -> Result<impl IntoResponse, StorageError> {
    let data = storage.get_object(&bucket, &object).await?;
    Ok(data)
}

// DELETE - 删除对象
async fn delete_object(
    State(storage): State<AppState>,
    Path((bucket, object)): Path<(String, String)>,
) -> Result<impl IntoResponse, StorageError> {
    storage.delete_object(&bucket, &object).await?;
    Ok(StatusCode::NO_CONTENT)
}
