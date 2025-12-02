use axum::{
    extract::FromRequestParts,
    http::{header, request::Parts},
};
use base64::{Engine, prelude::BASE64_STANDARD};
use bytes::Bytes;
use chrono::Utc;
use crab_vault::engine::ObjectMeta;
use crab_vault_engine::BucketMeta;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use crate::{
    error::api::{ApiError, ClientError},
    http::X_CRAB_VAULT_USER_META,
};

/// 从请求头中提取元数据，用于创建新的 ObjectMeta。
#[derive(Debug)]
pub struct ObjectMetaExtractor {
    pub bucket_name: String,
    pub object_name: String,
    pub content_type: String,
    pub user_meta: Value,
}

pub struct BuckeMetaExtractor {
    pub name: String,
    pub user_meta: Value,
}

impl<S> FromRequestParts<S> for ObjectMetaExtractor
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
            return Err(ApiError::Client(ClientError::UriInvalid));
        }

        let bucket_name = path_params[0].to_string();
        let object_name = path_params[1..].join("/");

        let content_type = parts
            .headers
            .get(header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            // octet-stream 是默认值，如果没有提供 content type
            // 按理说 AuthMiddleware 会拦截没有携带 content type 的请求
            .unwrap_or("application/octet-stream")
            .to_string();

        let user_meta = match parts.headers.get(X_CRAB_VAULT_USER_META) {
            Some(header_value) => {
                let raw_value = header_value.to_str()?;
                let decoded = BASE64_STANDARD.decode(raw_value)?;
                serde_json::from_slice(&decoded)?
            }
            None => json!({}),
        };

        Ok(Self {
            bucket_name,
            object_name,
            content_type,
            user_meta,
        })
    }
}

impl<S> FromRequestParts<S> for BuckeMetaExtractor
where
    S: Send + Sync,
{
    type Rejection = ApiError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let name = parts
            .uri
            .path()
            .split('/')
            .find(|s| !s.is_empty())
            .ok_or(ApiError::Client(ClientError::UriInvalid))?
            .to_string();

        let user_meta = match parts.headers.get(X_CRAB_VAULT_USER_META) {
            Some(header_value) => {
                let raw_value = header_value.to_str()?;
                let decoded = BASE64_STANDARD.decode(raw_value)?;
                serde_json::from_slice(&decoded)?
            }
            None => json!({}),
        };

        Ok(Self { name, user_meta })
    }
}

impl ObjectMetaExtractor {
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

impl BuckeMetaExtractor {
    pub fn into_meta(self) -> BucketMeta {
        let Self { name, user_meta } = self;
        BucketMeta::new(name, user_meta)
    }
}
