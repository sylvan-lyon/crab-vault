use axum::{
    debug_handler,
    extract::{Path, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};

use crate::http::{
    api::{
        ApiState,
        response::{BucketResponse, ObjectResponse},
        util::merge_json_object,
    },
    extractor::{auth::RestrictedBytes, meta::{BuckeMetaExtractor, ObjectMetaExtractor}},
};

use crab_vault::engine::{error::EngineResult, *};

// --- Bucket Handlers ---
#[debug_handler]
pub(super) async fn create_bucket(
    State(state): State<ApiState>,
    meta: BuckeMetaExtractor,
) -> EngineResult<StatusCode> {
    let meta = meta.into_meta();

    tracing::info!("{:?}", meta);

    // 操作是幂等的，所以我们不关心它们是否已经存在
    state.data_src.create_bucket(&meta.name).await?;
    state.meta_src.create_bucket_meta(&meta).await?;

    Ok(StatusCode::CREATED)
}

#[debug_handler]
pub(super) async fn delete_bucket(
    State(state): State<ApiState>,
    Path(bucket_name): Path<String>,
) -> EngineResult<StatusCode> {
    state.data_src.delete_bucket(&bucket_name).await?;
    state.meta_src.delete_bucket_meta(&bucket_name).await?;

    Ok(StatusCode::NO_CONTENT)
}

#[debug_handler]
pub(super) async fn head_bucket(
    State(state): State<ApiState>,
    Path(bucket_name): Path<String>,
) -> EngineResult<Response> {
    let meta = state.meta_src.read_bucket_meta(&bucket_name).await?;

    Ok(BucketResponse::new(meta).into_response())
}

#[debug_handler]
pub(super) async fn patch_bucket_meta(
    State(state): State<ApiState>,
    new: BuckeMetaExtractor,
) -> EngineResult<StatusCode> {
    let mut old_meta = state.meta_src.read_bucket_meta(&new.name).await?;
    old_meta.user_meta = merge_json_object(new.user_meta, old_meta.user_meta)?;
    state.meta_src.create_bucket_meta(&old_meta).await?;
    state.meta_src.touch_bucket(&new.name).await?;

    Ok(StatusCode::OK)
}

#[debug_handler]
pub(super) async fn list_buckets_meta(State(state): State<ApiState>) -> EngineResult<Response> {
    let res = state.meta_src.list_buckets_meta().await?;
    let res = res
        .into_iter()
        .map(BucketResponse::new)
        .collect::<Vec<_>>();

    Ok((StatusCode::OK, axum::Json(res)).into_response())
}

// --- Object Handlers ---

#[debug_handler]
pub(super) async fn upload_object(
    State(state): State<ApiState>,
    meta: ObjectMetaExtractor,
    RestrictedBytes(data): RestrictedBytes,
) -> EngineResult<StatusCode> {
    // 1. 检查 bucket 是否存在
    state
        .meta_src
        .read_bucket_meta(&meta.bucket_name)
        .await?;

    // 2. 从提取器和数据中创建完整的元数据
    let meta = meta.into_meta(&data);

    // 3. 原子地写入数据和元数据
    state
        .data_src
        .create_object(&meta.bucket_name, &meta.object_name, &data)
        .await?;
    state.meta_src.create_object_meta(&meta).await?;

    Ok(StatusCode::CREATED)
}

#[debug_handler]
pub(super) async fn get_object(
    State(state): State<ApiState>,
    Path((bucket_name, object_name)): Path<(String, String)>,
) -> EngineResult<ObjectResponse> {
    let meta = state
        .meta_src
        .read_object_meta(&bucket_name, &object_name)
        .await?;

    let data = state
        .data_src
        .read_object(&bucket_name, &object_name)
        .await?;

    Ok(ObjectResponse::new(meta, data))
}

#[debug_handler]
pub(super) async fn head_object(
    State(state): State<ApiState>,
    Path((bucket_name, object_name)): Path<(String, String)>,
) -> EngineResult<ObjectResponse> {
    let meta = state
        .meta_src
        .read_object_meta(&bucket_name, &object_name)
        .await?;

    Ok(ObjectResponse::meta_only(meta))
}

#[debug_handler]
pub(super) async fn patch_object_meta(
    State(state): State<ApiState>,
    Path((bucket_name, object_name)): Path<(String, String)>,
    new_meta: ObjectMetaExtractor,
) -> EngineResult<StatusCode> {
    let mut old_meta = state
        .meta_src
        .read_object_meta(&bucket_name, &object_name)
        .await?;

    old_meta.user_meta = merge_json_object(new_meta.user_meta, old_meta.user_meta)?;

    state.meta_src.create_object_meta(&old_meta).await?;
    state
        .meta_src
        .touch_object(&bucket_name, &object_name)
        .await?;

    Ok(StatusCode::OK)
}

#[debug_handler]
pub(super) async fn delete_object(
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
pub(super) async fn list_objects_meta(
    State(state): State<ApiState>,
    Path(bucket_name): Path<String>,
) -> EngineResult<Response> {
    let res = state.meta_src.list_objects_meta(&bucket_name).await?;

    Ok((StatusCode::OK, axum::Json(res)).into_response())
}

#[debug_handler]
pub(super) async fn health() -> Response {
    StatusCode::NO_CONTENT.into_response()
}
