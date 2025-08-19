use std::path::{Path, PathBuf};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    common::errors::StorageError,
    storage::{BucketMeta, DataStorage, MetaStorage, ObjectMeta},
};

pub struct FsDataStorage {
    base_dir: PathBuf,
}

impl FsDataStorage {
    fn object_data_path(&self, bucket_name: &str, object_name: &str) -> PathBuf {
        self.base_dir.join(bucket_name).join(object_name)
    }
}

impl DataStorage for FsDataStorage {
    fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self, StorageError> {
        let base_dir = base_dir.as_ref().to_path_buf();
        Ok(Self { base_dir })
    }

    async fn create_object(
        &self,
        bucket_name: &str,
        object_name: &str,
        data: &[u8],
    ) -> Result<(), StorageError> {
        let path = self.object_data_path(bucket_name, object_name);

        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;
        }

        // 异步写入文件
        let mut file = File::create(&path)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;

        file.write_all(data)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;

        file.flush()
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;

        Ok(())
    }

    async fn read_object(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> Result<Vec<u8>, StorageError> {
        let path = self.object_data_path(bucket_name, object_name);
        let mut file = File::open(&path)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;

        let mut contents = Vec::new();
        file.read_to_end(&mut contents)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;

        Ok(contents)
    }

    async fn delete_object(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> Result<(), StorageError> {
        let path = self.object_data_path(bucket_name, object_name);
        fs::remove_file(&path)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))
    }

    async fn head_object(&self, bucket_name: &str, object_name: &str) -> Result<u64, StorageError> {
        let path = self.object_data_path(bucket_name, object_name);
        let metadata = fs::metadata(&path)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;
        Ok(metadata.len())
    }
}

pub struct FsMetaStorage {
    base_dir: PathBuf,
}

impl FsMetaStorage {
    fn bucket_meta_path(&self, bucket_name: &str) -> PathBuf {
        self.base_dir.join(format!("{bucket_name}.json"))
    }

    fn object_meta_path(&self, bucket_name: &str, object_key: &str) -> PathBuf {
        self.base_dir
            .join(bucket_name)
            .join(format!("{object_key}.json"))
    }
}

impl MetaStorage for FsMetaStorage {
    fn new<P: AsRef<Path>>(base_dir: P) -> Result<Self, StorageError> {
        let base_dir = base_dir.as_ref().to_path_buf();
        Ok(Self { base_dir })
    }

    async fn put_bucket_meta(&self, bucket: &BucketMeta) -> Result<(), StorageError> {
        let path = self.bucket_meta_path(&bucket.name);

        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;
        }

        // 序列化并写入JSON
        let json = serde_json::to_string_pretty(bucket).map_err(StorageError::Serialization)?;

        fs::write(&path, json)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))
    }

    async fn get_bucket_meta(&self, name: &str) -> Result<BucketMeta, StorageError> {
        let path = self.bucket_meta_path(name);
        let data = fs::read_to_string(&path)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;

        serde_json::from_str(&data).map_err(StorageError::Deserialization)
    }

    async fn delete_bucket_meta(&self, name: &str) -> Result<(), StorageError> {
        let path = self.bucket_meta_path(name);
        fs::remove_file(&path)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))
    }

    async fn list_buckets_meta(&self) -> Result<Vec<BucketMeta>, StorageError> {
        let dir_path = self.base_dir.join("meta").join("buckets");
        let mut entries = fs::read_dir(&dir_path)
            .await
            .map_err(|e| StorageError::Io(e, dir_path.to_string_lossy().to_string()))?;

        let mut buckets = Vec::new();

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| StorageError::Io(e, dir_path.to_string_lossy().to_string()))?
        {
            if entry
                .file_type()
                .await
                .map_err(|e| StorageError::Io(e, dir_path.to_string_lossy().to_string()))?
                .is_file()
            {
                let path = entry.path();
                if let Some(ext) = path.extension()
                    && ext == "json"
                {
                    let data = fs::read_to_string(&path)
                        .await
                        .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;

                    let bucket: BucketMeta =
                        serde_json::from_str(&data).map_err(StorageError::Deserialization)?;

                    buckets.push(bucket);
                }
            }
        }

        Ok(buckets)
    }

    async fn put_object_meta(&self, metadata: &ObjectMeta) -> Result<(), StorageError> {
        let path = self.object_meta_path(&metadata.bucket_name, &metadata.object_name);

        // 创建父目录
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;
        }

        // 序列化并写入JSON
        let json = serde_json::to_string_pretty(metadata).map_err(StorageError::Serialization)?;

        fs::write(&path, json)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))
    }

    async fn get_object_meta(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> Result<ObjectMeta, StorageError> {
        let path = self.object_meta_path(bucket_name, object_key);
        let data = fs::read_to_string(&path)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;

        serde_json::from_str(&data).map_err(StorageError::Deserialization)
    }

    async fn delete_object_meta(
        &self,
        bucket_name: &str,
        object_key: &str,
    ) -> Result<(), StorageError> {
        let path = self.object_meta_path(bucket_name, object_key);
        fs::remove_file(&path)
            .await
            .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))
    }

    async fn list_objects_meta(&self, bucket_name: &str) -> Result<Vec<ObjectMeta>, StorageError> {
        let dir_path = self.base_dir.join("meta").join("objects").join(bucket_name);

        // 如果目录不存在，返回空列表
        if !dir_path.exists() {
            return Ok(Vec::new());
        }

        let mut entries = fs::read_dir(&dir_path)
            .await
            .map_err(|e| StorageError::Io(e, dir_path.to_string_lossy().to_string()))?;

        let mut objects = Vec::new();

        while let Some(entry) = entries
            .next_entry()
            .await
            .map_err(|e| StorageError::Io(e, dir_path.to_string_lossy().to_string()))?
        {
            if entry
                .file_type()
                .await
                .map_err(|e| StorageError::Io(e, dir_path.to_string_lossy().to_string()))?
                .is_file()
            {
                let path = entry.path();
                if let Some(ext) = path.extension()
                    && ext == "json"
                {
                    let data = fs::read_to_string(&path)
                        .await
                        .map_err(|e| StorageError::Io(e, path.to_string_lossy().to_string()))?;

                    let object: ObjectMeta =
                        serde_json::from_str(&data).map_err(StorageError::Deserialization)?;

                    objects.push(object);
                }
            }
        }

        Ok(objects)
    }
}
