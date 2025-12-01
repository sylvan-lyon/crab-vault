use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::error::EngineResult;

pub mod error;
pub mod fs;

pub type DataSource = fs::FsDataEngine;
pub type MetaSource = fs::FsMetaEngine;

/// Bucket 的元数据结构
#[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct BucketMeta {
    pub name: String,

    #[serde(alias = "createdAt")]
    pub created_at: DateTime<Utc>,

    #[serde(alias = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    pub user_meta: serde_json::Value,
}

/// Object 的元数据结构
#[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
#[serde(rename_all = "kebab-case")]
pub struct ObjectMeta {
    pub object_name: String,
    pub bucket_name: String,
    pub size: u64,
    pub content_type: String,
    pub etag: String,

    #[serde(alias = "createdAt")]
    pub created_at: DateTime<Utc>,

    #[serde(alias = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    pub user_meta: serde_json::Value,
}

/// 此 trait 定义了 object 从何处来，所有的操作，都是幂等的
pub trait DataEngine: Sized {
    type Uri: ?Sized;

    /// 创建一个新的实现了 [`DataEngine`] 的实例
    fn new<T: AsRef<Self::Uri>>(base_dir: T) -> EngineResult<Self>;

    /// 创建一个 bucket
    fn create_bucket(&self, bucket_name: &str) -> impl Future<Output = EngineResult<()>> + Send;

    /// 删除一个 bucket
    fn delete_bucket(&self, bucket_name: &str) -> impl Future<Output = EngineResult<()>> + Send;

    /// 创建一个 object
    fn create_object(
        &self,
        bucket_name: &str,
        object_name: &str,
        data: &[u8],
    ) -> impl Future<Output = EngineResult<()>> + Send;

    /// 读取一个 object
    fn read_object(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> impl Future<Output = EngineResult<Vec<u8>>> + Send;

    /// 删除一个 object
    fn delete_object(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> impl Future<Output = EngineResult<()>> + Send;
}

/// 此 trait 定义了 metadata 从何处来，所有的操作，都是幂等的
pub trait MetaEngine: Sized {
    type Uri: ?Sized;

    fn new<T: AsRef<Self::Uri>>(base_dir: T) -> EngineResult<Self>;

    // --- Bucket Operations ---

    /// 创建一个新的 Bucket 元数据
    fn create_bucket_meta(
        &self,
        meta: &BucketMeta,
    ) -> impl Future<Output = EngineResult<()>> + Send;

    /// 获取指定 Bucket 的元数据
    fn read_bucket_meta(
        &self,
        bucket_name: &str,
    ) -> impl Future<Output = EngineResult<BucketMeta>> + Send;

    /// 删除一个 Bucket 元数据 (要求 Bucket 为空)
    fn delete_bucket_meta(
        &self,
        bucket_name: &str,
    ) -> impl Future<Output = EngineResult<()>> + Send;

    /// 列出所有的 Bucket 的元数据
    fn list_buckets_meta(&self) -> impl Future<Output = EngineResult<Vec<BucketMeta>>> + Send;

    /// 更新一个 object 的 last_update 字段
    fn touch_object(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> impl Future<Output = EngineResult<()>> + Send;

    // --- Object Operations ---

    /// 存储（或更新）一个 Object 的元数据
    fn create_object_meta(
        &self,
        meta: &ObjectMeta,
    ) -> impl Future<Output = EngineResult<()>> + Send;

    /// 获取指定 Object 的元数据
    fn read_object_meta(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> impl Future<Output = EngineResult<ObjectMeta>> + Send;

    /// 删除一个 Object 的元数据
    fn delete_object_meta(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> impl Future<Output = EngineResult<()>> + Send;

    /// 列出指定 Bucket 内的所有 Object 元数据
    fn list_objects_meta(
        &self,
        bucket_name: &str,
    ) -> impl Future<Output = EngineResult<Vec<ObjectMeta>>> + Send;

    /// 更新一个 object 的 last_update 字段
    fn touch_bucket(&self, bucket_name: &str) -> impl Future<Output = EngineResult<()>> + Send;
}
