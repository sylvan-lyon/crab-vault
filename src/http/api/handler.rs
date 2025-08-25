use axum::{
    Json,
    body::Bytes,
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde_json::Value;

use crate::{
    error::engine::EngineResult,
    http::api::{
        ApiState,
        util::{BucketMetaResponse, NewObjectMetaExtractor, ObjectMetaResponse, merge_json_value},
    },
    storage::{BucketMeta, DataEngine, MetaEngine},
};

// --- Bucket Handlers ---
#[debug_handler]
pub async fn create_bucket(
    State(state): State<ApiState>,
    Path(bucket_name): Path<String>,
    Json(payload): Json<Value>, // User meta for the bucket
) -> EngineResult<StatusCode> {
    let meta = BucketMeta {
        name: bucket_name.clone(),
        created_at: chrono::Utc::now(),
        updated_at: chrono::Utc::now(),
        user_meta: payload,
    };

    // 操作是幂等的，所以我们不关心它们是否已经存在
    state.data_src.create_bucket(&bucket_name).await?;
    state.meta_src.create_bucket_meta(&meta).await?;

    Ok(StatusCode::CREATED)
}

#[debug_handler]
pub async fn delete_bucket(
    State(state): State<ApiState>,
    Path(bucket_name): Path<String>,
) -> EngineResult<StatusCode> {
    state.data_src.delete_bucket(&bucket_name).await?;
    state.meta_src.delete_bucket_meta(&bucket_name).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[debug_handler]
pub async fn head_bucket(
    State(state): State<ApiState>,
    Path(bucket_name): Path<String>,
) -> EngineResult<Response> {
    let meta = state.meta_src.read_bucket_meta(&bucket_name).await?;

    Ok(BucketMetaResponse::new(meta).into_response())
}

#[debug_handler]
pub async fn patch_bucket_meta(
    State(state): State<ApiState>,
    Path(bucket_name): Path<String>,
    Json(payload): Json<Value>,
) -> EngineResult<StatusCode> {
    let mut meta = state.meta_src.read_bucket_meta(&bucket_name).await?;
    meta.user_meta = merge_json_value(payload, meta.user_meta)?;
    state.meta_src.create_bucket_meta(&meta).await?;
    Ok(StatusCode::OK)
}

#[debug_handler]
pub async fn list_buckets_meta(State(state): State<ApiState>) -> EngineResult<Response> {
    let res = state.meta_src.list_buckets_meta().await?;
    let res = res
        .into_iter()
        .map(BucketMetaResponse::new)
        .collect::<Vec<_>>();

    Ok((StatusCode::OK, axum::Json(res)).into_response())
}

// --- Object Handlers ---

#[debug_handler]
pub async fn upload_object(
    State(state): State<ApiState>,
    meta_extractor: NewObjectMetaExtractor,
    data: Bytes,
) -> EngineResult<StatusCode> {
    // 1. 检查 bucket 是否存在
    state
        .meta_src
        .read_bucket_meta(&meta_extractor.bucket_name)
        .await?;

    // 2. 从提取器和数据中创建完整的元数据
    let meta = meta_extractor.into_meta(&data);

    // 3. 原子地写入数据和元数据
    state
        .data_src
        .create_object(&meta.bucket_name, &meta.object_name, &data)
        .await?;
    state.meta_src.create_object_meta(&meta).await?;

    Ok(StatusCode::CREATED)
}

#[debug_handler]
pub async fn get_object(
    State(state): State<ApiState>,
    Path((bucket_name, object_name)): Path<(String, String)>,
) -> EngineResult<ObjectMetaResponse> {
    let meta = state
        .meta_src
        .read_object_meta(&bucket_name, &object_name)
        .await?;
    let data = state
        .data_src
        .read_object(&bucket_name, &object_name)
        .await?;

    Ok(ObjectMetaResponse::new(meta, data))
}

#[debug_handler]
pub async fn head_object(
    State(state): State<ApiState>,
    Path((bucket_name, object_name)): Path<(String, String)>,
) -> EngineResult<ObjectMetaResponse> {
    let meta = state
        .meta_src
        .read_object_meta(&bucket_name, &object_name)
        .await?;

    Ok(ObjectMetaResponse::meta_only(meta))
}

#[debug_handler]
pub async fn patch_object_meta(
    State(state): State<ApiState>,
    Path((bucket_name, object_name)): Path<(String, String)>,
    Json(payload): Json<Value>,
) -> EngineResult<StatusCode> {
    let mut meta = state
        .meta_src
        .read_object_meta(&bucket_name, &object_name)
        .await?;

    meta.user_meta = merge_json_value(payload, meta.user_meta)?;
    meta.updated_at = chrono::Utc::now();

    state.meta_src.create_object_meta(&meta).await?;

    Ok(StatusCode::OK)
}

#[debug_handler]
pub async fn delete_object(
    State(state): State<ApiState>,
    Path((bucket_name, object_name)): Path<(String, String)>,
) -> EngineResult<StatusCode> {
    // 原子地删除数据和元数据
    state
        .data_src
        .delete_object(&bucket_name, &object_name)
        .await?;
    state
        .meta_src
        .delete_object_meta(&bucket_name, &object_name)
        .await?;

    Ok(StatusCode::NO_CONTENT)
}

#[debug_handler]
pub async fn list_objects_meta(
    State(state): State<ApiState>,
    Path(bucket_name): Path<String>,
) -> EngineResult<Response> {
    let res = state.meta_src.list_objects_meta(&bucket_name).await?;

    Ok((StatusCode::OK, axum::Json(res)).into_response())
}
