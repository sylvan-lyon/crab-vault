use axum::{
    http::{HeaderMap, HeaderName, HeaderValue, StatusCode, header},
    response::{IntoResponse, Response},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use crab_vault::engine::{BucketMeta, ObjectMeta};
use serde::Serialize;

/// 一个自定义的响应类型，它将元数据放入 Headers，数据放入 Body。
pub struct ObjectMetaResponse {
    meta: ObjectMeta,
    data: Option<Vec<u8>>, // Optional, because HEAD requests have no body
}

#[derive(Serialize)]
pub struct BucketMetaResponse {
    pub meta: BucketMeta,
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
            HeaderValue::from_bytes(&self.meta.updated_at.to_rfc2822().as_bytes()).unwrap(),
        );

        let headers = append_user_mata_to_headers(self.meta.user_meta, headers);

        (StatusCode::OK, headers).into_response()
    }
}

pub fn append_user_mata_to_headers(value: serde_json::Value, mut headers: HeaderMap) -> HeaderMap {
    if let Ok(header_name) = HeaderName::from_bytes("x-crab-vault-user-meta".as_bytes())
        && let Ok(json_string) = serde_json::to_string(&value)
        && let Ok(header_value) = HeaderValue::from_str(&BASE64_STANDARD.encode(json_string))
    {
        headers.insert(header_name, header_value);
    }

    headers
}
