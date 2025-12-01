use serde::de::DeserializeOwned;
use std::path::{Path, PathBuf};
use tokio::{
    fs::{self, File},
    io::{AsyncReadExt, AsyncWriteExt},
};

use crate::{
    error::{EngineError, EngineResult},
    {BucketMeta, DataEngine, MetaEngine, ObjectMeta},
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
    type Uri = Path;

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
            if (e.kind() == std::io::ErrorKind::DirectoryNotEmpty
                || e.kind() == std::io::ErrorKind::NotADirectory)
                && path.is_dir()
            {
                return Err(EngineError::BucketNotEmpty {
                    bucket: bucket_name.to_string(),
                });
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
    type Uri = Path;

    fn new<P: AsRef<Path>>(base_dir: P) -> EngineResult<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        // 在初始化时创建元数据根目录
        std::fs::create_dir_all(&base_dir).map_err(|e| io_error(e, &base_dir))?;
        Ok(Self { base_dir })
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

    async fn touch_object(&self, bucket_name: &str, object_name: &str) -> EngineResult<()> {
        let path = self.object_meta_path(bucket_name, object_name);

        match fs::read_to_string(&path).await {
            Ok(data) => {
                let mut meta: ObjectMeta = serde_json::from_str(&data)?;
                meta.updated_at = chrono::Utc::now();
                fs::write(&path, serde_json::to_string_pretty(&meta)?)
                    .await
                    .map_err(|e| io_error(e, &path))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(EngineError::ObjectMetaNotFound {
                    bucket: bucket_name.to_string(),
                    object: object_name.to_string(),
                })
            }
            Err(e) => Err(io_error(e, &path)),
        }
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

    async fn touch_bucket(&self, bucket_name: &str) -> EngineResult<()> {
        let path = self.bucket_meta_path(bucket_name);

        match fs::read_to_string(&path).await {
            Ok(data) => {
                let mut meta: BucketMeta = serde_json::from_str(&data)?;
                meta.updated_at = chrono::Utc::now();
                fs::write(&path, serde_json::to_string_pretty(&meta)?)
                    .await
                    .map_err(|e| io_error(e, &path))
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                Err(EngineError::BucketMetaNotFound {
                    bucket: bucket_name.to_string(),
                })
            }
            Err(e) => Err(io_error(e, &path)),
        }
    }

    async fn list_buckets_meta(&self) -> EngineResult<Vec<BucketMeta>> {
        let dir_path = self.buckets_dir_path();
        list_meta_from_dir(&dir_path).await
    }
}
