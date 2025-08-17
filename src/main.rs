mod error;
mod storage;

use axum::{
    Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, put},
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};
use std::{net::Ipv4Addr, sync::Arc, time::Duration};
use storage::FileStorage;
use tower_http::cors::{self, CorsLayer};

type AppState = Arc<FileStorage>;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_ansi(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_names(true)
                .with_thread_ids(false)
                .pretty(),
        )
        .init();

    let storage = FileStorage::new("./data").expect("Failed to create storage");
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

    let listener = tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, 32767))
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
) -> Result<impl IntoResponse, error::StorageError> {
    storage.put_object(&bucket, &object, &body).await?;
    Ok(StatusCode::CREATED)
}

// GET - 获取对象
async fn get_object(
    State(storage): State<AppState>,
    Path((bucket, object)): Path<(String, String)>,
) -> Result<impl IntoResponse, error::StorageError> {
    let data = storage.get_object(&bucket, &object).await?;
    Ok(data)
}

// DELETE - 删除对象
async fn delete_object(
    State(storage): State<AppState>,
    Path((bucket, object)): Path<(String, String)>,
) -> Result<impl IntoResponse, error::StorageError> {
    storage.delete_object(&bucket, &object).await?;
    Ok(StatusCode::NO_CONTENT)
}
