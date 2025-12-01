use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use bytes::Bytes;
use chrono::Utc;
use crab_vault::engine::ObjectMeta;
use serde_json::json;
use sha2::{Digest, Sha256};

use crate::{error::api::ApiError, http::USER_META_HEADER_KEY};

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
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        // 从路径中获取 bucket 和 object 名称
        let path_params: Vec<_> = parts
            .uri
            .path()
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();

        if path_params.len() < 2 {
            return Err(ApiError::UriInvalid);
        }

        let bucket_name = path_params[0].to_string();
        let object_name = path_params[1..].join("/");

        let content_type = (&parts.headers)
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            // octet-stream 是默认值，如果没有提供 content type
            .unwrap_or("application/octet-stream")
            .to_string();

        let user_meta = (&parts.headers)
            .get(USER_META_HEADER_KEY)
            // 由于是客户端传过来的，我们无视这些问题，全部用 ok() 给他撇了
            .and_then(|raw_value| raw_value.to_str().ok())
            .and_then(|b64_value| BASE64_STANDARD.decode(b64_value).ok())
            .and_then(|str_value| serde_json::from_slice(&str_value).ok())
            // 默认值设置为空 JSON 对象
            .unwrap_or(json!({}));

        Ok(Self {
            bucket_name,
            object_name,
            content_type,
            user_meta,
        })
    }
}

impl NewObjectMetaExtractor {
    /// 结合请求体数据，最终生成完整的 [`ObjectMeta`]
    pub fn into_meta(self, data: &Bytes) -> ObjectMeta {
        ObjectMeta {
            object_name: self.object_name,
            bucket_name: self.bucket_name,
            size: data.len() as u64,
            content_type: self.content_type,
            etag: BASE64_STANDARD.encode(Sha256::digest(data)),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            user_meta: self.user_meta,
        }
    }
}
