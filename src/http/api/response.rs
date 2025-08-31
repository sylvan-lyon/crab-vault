use axum::{http::{header, HeaderMap, HeaderValue, StatusCode}, response::{IntoResponse, Response}};
use crab_vault_engine::{BucketMeta, ObjectMeta};
use serde::Serialize;

use crate::{http::USER_META_PREFIX};



/// 一个自定义的响应类型，它将元数据放入 Headers，数据放入 Body。
pub struct ObjectMetaResponse {
    meta: ObjectMeta,
    data: Option<Vec<u8>>, // Optional, because HEAD requests have no body
}

impl ObjectMetaResponse {
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

impl IntoResponse for ObjectMetaResponse {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::CONTENT_TYPE,
            HeaderValue::from_str(&self.meta.content_type).unwrap(),
        );
        headers.insert(
            header::ETAG,
            HeaderValue::from_str(&self.meta.etag).unwrap(),
        );
        headers.insert(
            header::LAST_MODIFIED,
            HeaderValue::from_str(&self.meta.updated_at.to_rfc2822()).unwrap(),
        );

        let mut headers = append_user_mata_to_headers(self.meta.user_meta, headers);

        let body = self.data.unwrap_or_default();
        headers.insert(header::CONTENT_LENGTH, HeaderValue::from(body.len()));

        (StatusCode::OK, headers, body).into_response()
    }
}

#[derive(Serialize)]
pub struct BucketMetaResponse {
    pub meta: BucketMeta,
}

impl BucketMetaResponse {
    pub fn new(meta: BucketMeta) -> Self {
        Self { meta }
    }
}

impl IntoResponse for BucketMetaResponse {
    fn into_response(self) -> Response {
        let mut headers = HeaderMap::new();
        headers.insert(
            header::LAST_MODIFIED,
            HeaderValue::from_str(&self.meta.updated_at.to_rfc2822()).unwrap(),
        );

        let headers = append_user_mata_to_headers(self.meta.user_meta, headers);

        (StatusCode::OK, headers).into_response()
    }
}

pub fn append_user_mata_to_headers(value: serde_json::Value, mut headers: HeaderMap) -> HeaderMap {
    if let serde_json::Value::Object(map) = value {
        for (key, value) in map {
            if let Some(value_str) = value.as_str() {
                let header_key = format!("{}{}", USER_META_PREFIX, key);
                if let Ok(header_value) = HeaderValue::from_str(value_str) {
                    headers.insert(
                        axum::http::HeaderName::from_bytes(header_key.as_bytes()).unwrap(),
                        header_value,
                    );
                }
            }
        }
    }
    headers
}
