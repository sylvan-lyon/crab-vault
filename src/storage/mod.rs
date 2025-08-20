mod fs;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::common::errors::EngineResult;

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

// impl ObjectMeta {
//     /// 更新老的 [`ObjectMeta`]，而不是重新创建一个新的
//     /// 
//     /// 也就是说：除了 created_at 字段，其他的全部置为 self 中的数据
//     pub fn update(self, old: ObjectMeta) -> ObjectMeta {
//         ObjectMeta {
//             created_at: old.created_at,
//             ..self
//         }
//     }
// }

/// 此 trait 定义了 object 从何处来
pub trait DataEngine: Sized {
    /// 创建一个新的实现了 [`DataEngine`] 的实例
    fn new<P: AsRef<Path>>(base_dir: P) -> EngineResult<Self>;

    /// 创建一个 bucket，此操作是幂等的
    async fn create_bucket(&self, bucket_name: &str) -> EngineResult<()>;

    /// 删除一个 bucket，此操作是幂等的
    async fn delete_bucket(&self, bucket_name: &str) -> EngineResult<()>;

    /// 创建一个 object，此操作是幂等的
    async fn create_object(
        &self,
        bucket_name: &str,
        object_name: &str,
        data: &[u8],
    ) -> EngineResult<()>;

    /// 读取一个 object，此操作是幂等的
    async fn read_object(&self, bucket_name: &str, object_name: &str) -> EngineResult<Vec<u8>>;

    /// 删除一个 object，此操作是幂等的
    async fn delete_object(&self, bucket_name: &str, object_name: &str) -> EngineResult<()>;
}

/// 此 trait 定义了 metadata 从何处来
pub trait MetaEngine: Sized {
    fn new<P: AsRef<Path>>(base_dir: P) -> EngineResult<Self>;

    // --- Bucket Operations ---

    /// 创建一个新的 Bucket 元数据
    async fn create_bucket_meta(&self, meta: &BucketMeta) -> EngineResult<()>;

    /// 获取指定 Bucket 的元数据
    async fn read_bucket_meta(&self, bucket_name: &str) -> EngineResult<BucketMeta>;

    /// 删除一个 Bucket 元数据 (要求 Bucket 为空)
    async fn delete_bucket_meta(&self, bucket_name: &str) -> EngineResult<()>;

    /// 列出所有的 Bucket 的元数据
    async fn list_buckets_meta(&self) -> EngineResult<Vec<BucketMeta>>;

    // --- Object Operations ---

    /// 存储（或更新）一个 Object 的元数据
    async fn create_object_meta(&self, meta: &ObjectMeta) -> EngineResult<()>;

    /// 获取指定 Object 的元数据
    async fn read_object_meta(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> EngineResult<ObjectMeta>;

    /// 删除一个 Object 的元数据
    async fn delete_object_meta(&self, bucket_name: &str, object_name: &str) -> EngineResult<()>;

    /// 列出指定 Bucket 内的所有 Object 元数据
    async fn list_objects_meta(&self, bucket_name: &str) -> EngineResult<Vec<ObjectMeta>>;
}
