use axum::{
    body::Bytes,
    extract::FromRequestParts,
    http::{HeaderMap, HeaderValue, StatusCode, header, request::Parts},
    response::{IntoResponse, Response},
};
use base64::{Engine, prelude::BASE64_STANDARD_NO_PAD};
use chrono::Utc;
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{
    errors::engine::{EngineError, EngineResult},
    storage::{BucketMeta, ObjectMeta},
};

const USER_META_PREFIX: &str = "x-crab-vault-meta-";

/// 从请求头中提取元数据，用于创建新的 ObjectMeta。
#[derive(Debug)]
pub struct NewObjectMetaExtractor {
    pub bucket_name: String,
    pub object_name: String,
    pub content_type: String,
    pub user_meta: serde_json::Value,
}

impl<S> FromRequestParts<S> for NewObjectMetaExtractor
where
    S: Send + Sync,
{
    type Rejection = EngineError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 从路径中获取 bucket 和 object 名称
        let path_params: Vec<&str> = parts
            .uri
            .path()
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        if path_params.len() < 2 {
            return Err(EngineError::InvalidArgument(
                "Invalid path format. Expected /{bucket}/{object}".to_string(),
            ));
        }
        let bucket_name = path_params[0].to_string();
        let object_name = path_params[1..].join("/");

        let headers = &parts.headers;
        let content_type = headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let mut user_meta_map = serde_json::Map::new();
        for (key, value) in headers.iter() {
            if let Some(key_str) = key.as_str().strip_prefix(USER_META_PREFIX)
                && let Ok(value_str) = value.to_str() {
                    user_meta_map.insert(key_str.to_string(), json!(value_str));
                }
        }

        Ok(Self {
            bucket_name,
            object_name,
            content_type,
            user_meta: user_meta_map.into(),
        })
    }
}

impl NewObjectMetaExtractor {
    /// 结合请求体数据，最终生成完整的 ObjectMeta
    pub fn into_meta(self, data: &Bytes) -> ObjectMeta {
        ObjectMeta {
            object_name: self.object_name,
            bucket_name: self.bucket_name,
            size: data.len() as u64,
            content_type: self.content_type,
            etag: BASE64_STANDARD_NO_PAD.encode(Sha256::digest(data)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_meta: self.user_meta,
        }
    }
}

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

pub fn merge_json_value(
    new: serde_json::Value,
    old: serde_json::Value,
) -> EngineResult<serde_json::Value> {
    use serde_json::Value;

    let helper = |value: Value| match value {
        Value::Object(map) => Ok(map),
        _ => Err(EngineError::InvalidArgument(
            "Should be an object".to_string(),
        )),
    };

    // 首先确保新的值必须合法，否则返回一个 invalid argument 错误
    let new = helper(new)?;

    // 如果旧的值不合法，那么直接返回合法的新值
    // 将旧值作为基底
    let mut res = match helper(old) {
        Ok(val) => val,
        Err(_) => return Ok(Value::Object(new)),
    };

    for (k, v) in new {
        match v {
            Value::Null => res.remove(&k),
            _ => res.insert(k, v),
        };
    }

    Ok(Value::Object(res))
}
