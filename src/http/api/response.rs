use axum::{
    http::{
        HeaderMap, HeaderValue, StatusCode,
        header::{self, CONTENT_TYPE, ETAG, LAST_MODIFIED},
    },
    response::{IntoResponse, Response},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use crab_vault::engine::{BucketMeta, ObjectMeta};
use serde::Serialize;

use crate::http::{
    X_CRAB_VAULT_BUCKET_NAME, X_CRAB_VAULT_CREATED_AT, X_CRAB_VAULT_OBJECT_NAME,
    X_CRAB_VAULT_USER_META,
};

/// 一个自定义的响应类型，它将元数据放入 Headers，数据放入 Body。
pub struct ObjectResponse {
    meta: ObjectMeta,
    data: Option<Vec<u8>>, // Optional, because HEAD requests have no body
}

#[derive(Serialize)]
pub struct BucketResponse {
    meta: BucketMeta,
}

impl ObjectResponse {
    pub fn new(meta: ObjectMeta, data: Vec<u8>) -> Self {
        Self {
            meta,
            data: Some(data),
        }
    }
    pub fn meta_only(meta: ObjectMeta) -> Self {
        Self { meta, data: None }
    }
}

impl IntoResponse for ObjectResponse {
    fn into_response(self) -> Response {
        let Self { meta, data } = self;
        let ObjectMeta {
            object_name,
            bucket_name,
            size,
            content_type,
            etag,
            user_meta,
            created_at,
            updated_at,
        } = meta;

        let mut headers = HeaderMap::new();

        headers.insert(LAST_MODIFIED, HeaderValue::from(size));

        HeaderValue::from_str(&content_type)
            .ok()
            .and_then(|content_type| headers.insert(CONTENT_TYPE, content_type));

        HeaderValue::from_str(&etag)
            .ok()
            .and_then(|etag| headers.insert(ETAG, etag));

        HeaderValue::from_str(&updated_at.to_rfc2822())
            .ok()
            .and_then(|last_modified| headers.insert(LAST_MODIFIED, last_modified));

        HeaderValue::from_str(&created_at.to_rfc2822())
            .ok()
            .and_then(|created_at| headers.insert(X_CRAB_VAULT_CREATED_AT, created_at));

        HeaderValue::from_str(&object_name)
            .ok()
            .and_then(|object_name| headers.insert(X_CRAB_VAULT_OBJECT_NAME, object_name));

        HeaderValue::from_str(&bucket_name)
            .ok()
            .and_then(|bucket_name| headers.insert(X_CRAB_VAULT_BUCKET_NAME, bucket_name));

        let mut headers = append_user_mata_to_headers(user_meta, headers);

        let body = data.unwrap_or_default();
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from(body.len()));

        (StatusCode::OK, headers, body).into_response()
    }
}

impl BucketResponse {
    pub fn new(meta: BucketMeta) -> Self {
        Self { meta }
    }
}

impl IntoResponse for BucketResponse {
    fn into_response(self) -> Response {
        let BucketResponse { meta } = self;
        let BucketMeta {
            name,
            user_meta,
            created_at,
            updated_at,
        } = meta;

        let mut headers = HeaderMap::new();

        HeaderValue::from_str(&updated_at.to_rfc2822())
            .ok()
            .and_then(|last_modified| headers.insert(LAST_MODIFIED, last_modified));

        HeaderValue::from_str(&name)
            .ok()
            .and_then(|name| headers.insert(X_CRAB_VAULT_BUCKET_NAME, name));

        HeaderValue::from_str(&created_at.to_rfc2822())
            .ok()
            .and_then(|created_at| headers.insert(X_CRAB_VAULT_CREATED_AT, created_at));

        let headers = append_user_mata_to_headers(user_meta, headers);

        (StatusCode::OK, headers).into_response()
    }
}

pub fn append_user_mata_to_headers(value: serde_json::Value, mut headers: HeaderMap) -> HeaderMap {
    if let Ok(value_json_string) = serde_json::to_string(&value)
        && let Ok(header_value) = HeaderValue::from_str(&BASE64_STANDARD.encode(value_json_string))
    {
        headers.insert(X_CRAB_VAULT_USER_META, header_value);
    }

    headers
}
