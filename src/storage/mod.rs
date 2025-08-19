mod fs;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::common::errors::StorageError;

pub type DataSource = fs::FsDataStorage;
pub type MetaSource = fs::FsMetaStorage;

/// Bucket 的元数据结构
#[derive(Serialize, Deserialize)]
pub struct BucketMeta {
    #[serde(skip_deserializing)]
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_meta: serde_json::Value,
}

/// Object 的元数据结构
#[derive(Serialize, Deserialize)]
pub struct ObjectMeta {
    #[serde(skip_deserializing)]
    pub object_name: String,
    #[serde(skip_deserializing)]
    pub bucket_name: String,
    pub size: u64,
    pub content_type: String,
    pub etag: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub user_meta: serde_json::Value,
}

/// 此 trait 定义了 object 从何处来
pub trait DataStorage: Sized {
    fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self, StorageError>;

    async fn create_object(
        &self,
        bucket_name: &str,
        object_name: &str,
        data: &[u8],
    ) -> Result<(), StorageError>;

    async fn read_object(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> Result<Vec<u8>, StorageError>;

    async fn delete_object(&self, bucket_name: &str, object_name: &str)
    -> Result<(), StorageError>;

    async fn head_object(&self, bucket_name: &str, object_key: &str) -> Result<u64, StorageError>;
}

/// 此 trait 定义了 metadata 从何处来
pub trait MetaStorage: Sized {
    fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self, StorageError>;

    /// 创建一个新的 Bucket 元数据
    async fn put_bucket_meta(&self, bucket_name: &BucketMeta) -> Result<(), StorageError>;

    /// 获取指定 Bucket 的元数据
    async fn get_bucket_meta(&self, bucket_name: &str) -> Result<BucketMeta, StorageError>;

    /// 删除一个 Bucket 元数据 (通常要求 Bucket 为空)
    async fn delete_bucket_meta(&self, bucket_name: &str) -> Result<(), StorageError>;

    /// 列出所有的 Bucket 的元数据
    async fn list_buckets_meta(&self) -> Result<Vec<BucketMeta>, StorageError>;

    // --- Object Operations ---

    /// 存储（或更新）一个 Object 的元数据
    async fn put_object_meta(&self, metadata: &ObjectMeta) -> Result<(), StorageError>;

    /// 获取指定 Object 的元数据
    async fn get_object_meta(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> Result<ObjectMeta, StorageError>;

    /// 删除一个 Object 的元数据
    async fn delete_object_meta(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> Result<(), StorageError>;

    /// 列出指定 Bucket 内的所有 Object 元数据
    async fn list_objects_meta(&self, bucket_name: &str) -> Result<Vec<ObjectMeta>, StorageError>;
}
