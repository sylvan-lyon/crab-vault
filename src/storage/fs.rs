use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    errors::engine::{EngineError, EngineResult},
    storage::{BucketMeta, DataEngine, MetaEngine, ObjectMeta},
};

pub struct FsDataEngine {
    base_dir: PathBuf,
}

impl FsDataEngine {
    fn path_of_object(&self, bucket_name: &str, object_name: &str) -> PathBuf {
        self.base_dir.join(bucket_name).join(object_name)
    }

    fn path_of_bucket(&self, bucket_name: &str) -> PathBuf {
        self.base_dir.join(bucket_name)
    }
}

/// helper function，将 [IO Error](std::io::Error) 转换为 [`StorageError`]
#[inline(always)]
fn io_error<P: AsRef<Path> + ?Sized>(e: std::io::Error, path: &P) -> EngineError {
    EngineError::Io {
        error: e,
        path: path.as_ref().to_string_lossy().to_string(),
    }
}

impl DataEngine for FsDataEngine {
    fn new<P: AsRef<Path>>(base_dir: P) -> EngineResult<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        std::fs::create_dir_all(&base_dir).map_err(|e| io_error(e, &base_dir))?;
        Ok(Self { base_dir })
    }

    async fn create_bucket(&self, bucket_name: &str) -> EngineResult<()> {
        let path = self.path_of_bucket(bucket_name);

        fs::create_dir_all(&path)
            .await
            .map_err(|e| io_error(e, &path))?;

        Ok(())
    }

    async fn delete_bucket(&self, bucket_name: &str) -> EngineResult<()> {
        let path = self.path_of_bucket(bucket_name);

        // 直接尝试删除目录
        if let Err(e) = fs::remove_dir(&path).await {
            if e.kind() == std::io::ErrorKind::DirectoryNotEmpty
                || e.kind() == std::io::ErrorKind::NotADirectory
            {
                if path.is_dir() {
                    return Err(EngineError::BucketNotEmpty {
                        bucket: bucket_name.to_string(),
                    });
                }
            }
            // 对于其他类型的 IO 错误，正常地返回
            return Err(io_error(e, &path));
        }

        Ok(())
    }

    async fn create_object(
        &self,
        bucket_name: &str,
        object_name: &str,
        data: &[u8],
    ) -> EngineResult<()> {
        let path = self.path_of_object(bucket_name, object_name);

        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            return Err(EngineError::BucketNotFound {
                bucket: bucket_name.to_string(),
            });
        }

        // 异步写入文件
        let mut file = File::create(&path).await.map_err(|e| io_error(e, &path))?;
        file.write_all(data).await.map_err(|e| io_error(e, &path))?;
        file.flush().await.map_err(|e| io_error(e, &path))?;

        Ok(())
    }

    async fn read_object(&self, bucket_name: &str, object_name: &str) -> EngineResult<Vec<u8>> {
        let path = self.path_of_object(bucket_name, object_name);
        let map_io_err = |e| io_error(e, &path);

        // 直接尝试打开文件，并处理 NotFound 错误
        let mut file = match File::open(&path).await {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                return Err(EngineError::ObjectNotFound {
                    bucket: bucket_name.to_string(),
                    object: object_name.to_string(),
                });
            }
            Err(e) => return Err(map_io_err(e)),
        };

        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await.map_err(map_io_err)?;

        Ok(contents)
    }

    async fn delete_object(&self, bucket_name: &str, object_name: &str) -> EngineResult<()> {
        let path = self.path_of_object(bucket_name, object_name);

        match fs::remove_file(&path).await {
            Ok(_) => Ok(()),
            // 如果文件不存在，我们认为删除操作是成功的（幂等性）
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_error(e, &path)),
        }
    }
}

pub struct FsMetaEngine {
    base_dir: PathBuf,
}

impl FsMetaEngine {
    // 优化的路径结构
    fn bucket_meta_path(&self, bucket_name: &str) -> PathBuf {
        self.base_dir
            .join("buckets")
            .join(format!("{}.json", bucket_name))
    }

    fn object_meta_path(&self, bucket_name: &str, object_name: &str) -> PathBuf {
        self.base_dir
            .join("objects")
            .join(bucket_name)
            .join(format!("{}.json", object_name))
    }

    // 获取对象元数据目录的路径
    fn objects_dir_path(&self, bucket_name: &str) -> PathBuf {
        self.base_dir.join("objects").join(bucket_name)
    }

    // 获取 bucket 元数据目录的路径
    fn buckets_dir_path(&self) -> PathBuf {
        self.base_dir.join("buckets")
    }
}

/// 辅助函数，用于从目录中列出并反序列化所有JSON元数据文件。
async fn list_meta_from_dir<T: DeserializeOwned>(dir_path: &Path) -> EngineResult<Vec<T>> {
    // 如果目录不存在，这是一个正常情况，只返回一个空列表。
    if !dir_path.exists() {
        return Ok(Vec::new());
    }

    let mut entries = fs::read_dir(dir_path)
        .await
        .map_err(|e| io_error(e, dir_path))?;

    let mut results = Vec::new();

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| io_error(e, dir_path))?
    {
        let path = entry.path();
        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("json") {
            let data = fs::read_to_string(&path)
                .await
                .map_err(|e| io_error(e, &path))?;
            // 如果单个文件损坏，我们可以选择跳过它或返回错误。这里我们选择失败。
            let meta: T = serde_json::from_str(&data)?;
            results.push(meta);
        }
    }

    Ok(results)
}

impl MetaEngine for FsMetaEngine {
    fn new<P: AsRef<Path>>(base_dir: P) -> EngineResult<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        // 在初始化时创建元数据根目录
        std::fs::create_dir_all(&base_dir).map_err(|e| io_error(e, &base_dir))?;
        Ok(Self { base_dir })
    }

    async fn create_bucket_meta(&self, meta: &BucketMeta) -> EngineResult<()> {
        let path = self.bucket_meta_path(&meta.name);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| io_error(e, parent))?;
        }

        let json = serde_json::to_string_pretty(meta)?;
        fs::write(&path, json).await.map_err(|e| io_error(e, &path))
    }

    async fn read_bucket_meta(&self, name: &str) -> EngineResult<BucketMeta> {
        let path = self.bucket_meta_path(name);
        match fs::read_to_string(&path).await {
            Ok(data) => Ok(serde_json::from_str(&data)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(EngineError::BucketMetaNotFound {
                    bucket: name.to_string(),
                })
            }
            Err(e) => Err(io_error(e, &path)),
        }
    }

    async fn delete_bucket_meta(&self, name: &str) -> EngineResult<()> {
        let path = self.bucket_meta_path(name);
        match fs::remove_file(&path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_error(e, &path)),
        }?;

        match fs::remove_dir(self.objects_dir_path(name)).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_error(e, &path)),
        }?;

        Ok(())
    }

    async fn list_buckets_meta(&self) -> EngineResult<Vec<BucketMeta>> {
        let dir_path = self.buckets_dir_path();
        list_meta_from_dir(&dir_path).await
    }

    async fn create_object_meta(&self, meta: &ObjectMeta) -> EngineResult<()> {
        let path = self.object_meta_path(&meta.bucket_name, &meta.object_name);

        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .await
                .map_err(|e| io_error(e, parent))?;
        }

        let json = serde_json::to_string_pretty(meta)?;
        fs::write(&path, json).await.map_err(|e| io_error(e, &path))
    }

    async fn read_object_meta(
        &self,
        bucket_name: &str,
        object_name: &str,
    ) -> EngineResult<ObjectMeta> {
        let path = self.object_meta_path(bucket_name, object_name);
        match fs::read_to_string(&path).await {
            Ok(data) => Ok(serde_json::from_str(&data)?),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(EngineError::ObjectMetaNotFound {
                    bucket: bucket_name.to_string(),
                    object: object_name.to_string(),
                })
            }
            Err(e) => Err(io_error(e, &path)),
        }
    }

    async fn delete_object_meta(&self, bucket_name: &str, object_name: &str) -> EngineResult<()> {
        let path = self.object_meta_path(bucket_name, object_name);
        match fs::remove_file(&path).await {
            Ok(_) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(e) => Err(io_error(e, &path)),
        }
    }

    async fn list_objects_meta(&self, bucket_name: &str) -> EngineResult<Vec<ObjectMeta>> {
        let dir_path = self.objects_dir_path(bucket_name);
        list_meta_from_dir(&dir_path).await
    }
}

#[cfg(test)]
mod data_engine_tests {
    use super::*;
    use crate::errors::engine::EngineError;
    use std::path::PathBuf;

    const TEST_BASE_DIR: &str = "./data_test";

    async fn setup(test_name: &str) -> (FsDataEngine, PathBuf) {
        let base_dir = PathBuf::from(TEST_BASE_DIR).join(test_name);

        if base_dir.exists() {
            tokio::fs::remove_dir_all(&base_dir).await.unwrap();
        }

        let storage = FsDataEngine::new(&base_dir).expect("无法创建根文件夹");

        (storage, base_dir)
    }

    #[tokio::test]
    async fn test_new_creates_base_directory() {
        let test_name = "new_creates_dir";
        let base_dir = PathBuf::from(TEST_BASE_DIR).join(test_name);

        if base_dir.exists() {
            tokio::fs::remove_dir_all(&base_dir).await.unwrap();
        }

        assert!(!base_dir.exists());
        let _storage = FsDataEngine::new(&base_dir).unwrap();
        assert!(base_dir.exists());

        tokio::fs::remove_dir_all(&base_dir).await.unwrap();
    }

    #[tokio::test]
    async fn test_create_bucket_success() {
        let (storage, base_dir) = setup("create_bucket_success").await;
        let bucket_name = "test-bucket";

        let result = storage.create_bucket(bucket_name).await;
        assert!(result.is_ok());

        let bucket_path = base_dir.join(bucket_name);
        assert!(bucket_path.exists());
        assert!(bucket_path.is_dir());
    }

    #[tokio::test]
    async fn test_full_lifecycle() {
        let (storage, _base_dir) = setup("full_lifecycle").await;
        let bucket_name = "my-bucket";
        let object_name = "my-object";
        let data = b"hello world";

        storage.create_bucket(bucket_name).await.unwrap();

        storage
            .create_object(bucket_name, object_name, data)
            .await
            .unwrap();

        let read_data = storage.read_object(bucket_name, object_name).await.unwrap();
        assert_eq!(read_data, data);

        storage
            .delete_object(bucket_name, object_name)
            .await
            .unwrap();

        storage.delete_bucket(bucket_name).await.unwrap();

        let bucket_path = _base_dir.join(bucket_name);
        assert!(!bucket_path.exists());
    }

    #[tokio::test]
    async fn test_delete_non_empty_bucket_fails() {
        let (storage, _base_dir) = setup("delete_non_empty_bucket").await;
        let bucket_name = "non-empty-bucket";
        let object_name = "some-file.txt";

        storage.create_bucket(bucket_name).await.unwrap();
        storage
            .create_object(bucket_name, object_name, b"some data")
            .await
            .unwrap();

        let result = storage.delete_bucket(bucket_name).await;
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EngineError::BucketNotEmpty { bucket: _ })
        ));

        // Verificar se o bucket ainda existe
        let bucket_path = _base_dir.join(bucket_name);
        assert!(bucket_path.exists());
    }

    #[tokio::test]
    async fn test_create_object_in_nonexistent_bucket_fails() {
        let (storage, _base_dir) = setup("create_object_no_bucket").await;
        let bucket_name = "non-existent-bucket";
        let object_name = "some-object";

        let result = storage
            .create_object(bucket_name, object_name, b"data")
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EngineError::BucketNotFound { bucket: _ })
        ));
    }

    #[tokio::test]
    async fn test_read_nonexistent_object_fails() {
        let (storage, _base_dir) = setup("read_nonexistent_object").await;
        let bucket_name = "bucket";
        storage.create_bucket(bucket_name).await.unwrap();

        let result = storage
            .read_object(bucket_name, "non-existent-object")
            .await;
        assert!(result.is_err());
        assert!(matches!(
            result,
            Err(EngineError::ObjectNotFound {
                bucket: _,
                object: _
            })
        ));
    }

    #[tokio::test]
    async fn test_delete_nonexistent_object_is_ok() {
        let (storage, _base_dir) = setup("delete_nonexistent_object").await;
        let bucket_name = "bucket";
        storage.create_bucket(bucket_name).await.unwrap();

        // Tentar deletar um objeto que não existe deve ser bem-sucedido (idempotente)
        let result = storage
            .delete_object(bucket_name, "non-existent-object")
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_overwrite_object() {
        let (storage, _base_dir) = setup("overwrite_object").await;
        let bucket_name = "bucket";
        let object_name = "file.txt";
        let initial_data = b"initial version";
        let new_data = b"new version of the data";

        storage.create_bucket(bucket_name).await.unwrap();
        storage
            .create_object(bucket_name, object_name, initial_data)
            .await
            .unwrap();

        // Verificar o conteúdo inicial
        let read_data1 = storage.read_object(bucket_name, object_name).await.unwrap();
        assert_eq!(read_data1, initial_data);

        // Sobrescrever o objeto
        storage
            .create_object(bucket_name, object_name, new_data)
            .await
            .unwrap();

        // Verificar o novo conteúdo
        let read_data2 = storage.read_object(bucket_name, object_name).await.unwrap();
        assert_eq!(read_data2, new_data);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{BucketMeta, ObjectMeta}; // 确保路径正确
    use std::path::PathBuf;

    const TEST_META_BASE_DIR: &str = "./meta_test";

    async fn setup(test_name: &str) -> (FsMetaEngine, PathBuf) {
        let base_dir = PathBuf::from(TEST_META_BASE_DIR).join(test_name);
        if base_dir.exists() {
            tokio::fs::remove_dir_all(&base_dir).await.unwrap();
        }
        let storage = FsMetaEngine::new(&base_dir).expect("Failed to create test meta storage");
        (storage, base_dir)
    }

    #[tokio::test]
    async fn test_put_and_get_bucket_meta() {
        let (storage, base_dir) = setup("put_get_bucket").await;
        let bucket_meta = BucketMeta {
            name: "test-bucket".to_string(),
            ..BucketMeta::default()
        };

        // Put
        storage.create_bucket_meta(&bucket_meta).await.unwrap();

        // Verify file exists at the new path
        let expected_path = base_dir.join("buckets").join("test-bucket.json");
        assert!(expected_path.exists());

        // Get
        let fetched_meta = storage.read_bucket_meta("test-bucket").await.unwrap();
        println!("{}", fetched_meta.name);
        assert_eq!(bucket_meta, fetched_meta);
    }

    #[tokio::test]
    async fn test_get_nonexistent_bucket_meta_fails_correctly() {
        let (storage, _) = setup("get_nonexistent_bucket").await;
        let result = storage.read_bucket_meta("nonexistent-bucket").await;
        assert!(result.is_err());
        // 在这里检查您的自定义错误类型
        // assert!(matches!(result, Err(StorageError::BucketMetaNotFound(_))));
    }

    #[tokio::test]
    async fn test_delete_bucket_meta() {
        let (storage, _) = setup("delete_bucket").await;
        let bucket_meta = BucketMeta {
            name: "bucket-to-delete".to_string(),
            ..BucketMeta::default()
        };

        storage.create_bucket_meta(&bucket_meta).await.unwrap();
        assert!(storage.read_bucket_meta("bucket-to-delete").await.is_ok());

        // Delete
        storage
            .delete_bucket_meta("bucket-to-delete")
            .await
            .unwrap();
        assert!(storage.read_bucket_meta("bucket-to-delete").await.is_err());

        // Deleting again should be OK
        storage
            .delete_bucket_meta("bucket-to-delete")
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_list_buckets_meta() {
        let (storage, _) = setup("list_buckets").await;

        // Initially empty
        let buckets = storage.list_buckets_meta().await.unwrap();
        assert!(buckets.is_empty());

        // Add two buckets
        let bucket1 = BucketMeta {
            name: "bucket1".to_string(),
            ..BucketMeta::default()
        };
        let bucket2 = BucketMeta {
            name: "bucket2".to_string(),
            ..BucketMeta::default()
        };
        storage.create_bucket_meta(&bucket1).await.unwrap();
        storage.create_bucket_meta(&bucket2).await.unwrap();

        let mut buckets = storage.list_buckets_meta().await.unwrap();
        // Sort for deterministic comparison
        buckets.sort_by(|a, b| a.name.cmp(&b.name));

        assert_eq!(buckets.len(), 2);
        assert_eq!(buckets[0], bucket1);
        assert_eq!(buckets[1], bucket2);
    }

    #[tokio::test]
    async fn test_full_object_meta_lifecycle() {
        let (storage, base_dir) = setup("object_lifecycle").await;
        let bucket_name = "my-bucket";
        let object_meta1 = ObjectMeta {
            bucket_name: bucket_name.to_string(),
            object_name: "obj1".to_string(),
            ..ObjectMeta::default()
        };
        let object_meta2 = ObjectMeta {
            bucket_name: bucket_name.to_string(),
            object_name: "obj2".to_string(),
            ..ObjectMeta::default()
        };

        // 1. Put object meta
        storage.create_object_meta(&object_meta1).await.unwrap();
        storage.create_object_meta(&object_meta2).await.unwrap();

        // Verify file structure
        let expected_path = base_dir.join("objects").join(bucket_name).join("obj1.json");
        assert!(expected_path.exists());

        // 2. Get object meta
        let fetched_obj1 = storage.read_object_meta(bucket_name, "obj1").await.unwrap();
        assert_eq!(fetched_obj1, object_meta1);

        // 3. List objects meta
        let mut objects = storage.list_objects_meta(bucket_name).await.unwrap();
        objects.sort_by(|a, b| a.object_name.cmp(&b.object_name));
        assert_eq!(objects.len(), 2);
        assert_eq!(objects[0], object_meta1);
        assert_eq!(objects[1], object_meta2);

        // 4. Delete one object meta
        storage
            .delete_object_meta(bucket_name, "obj1")
            .await
            .unwrap();
        assert!(storage.read_object_meta(bucket_name, "obj1").await.is_err());

        // 5. List again
        let objects = storage.list_objects_meta(bucket_name).await.unwrap();
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0], object_meta2);
    }

    #[tokio::test]
    async fn test_list_objects_from_nonexistent_bucket_returns_empty() {
        let (storage, _) = setup("list_empty_bucket_objects").await;
        let objects = storage
            .list_objects_meta("nonexistent-bucket")
            .await
            .unwrap();
        assert!(objects.is_empty());
    }
}
