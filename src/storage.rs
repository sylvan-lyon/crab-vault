use crate::common::errors::StorageError;
use std::{
    fs, io,
    path::{Path, PathBuf},
};

pub struct FileStorage {
    base_dir: PathBuf,
}

impl FileStorage {
    pub fn new<P: AsRef<Path>>(base_dir: P) -> io::Result<Self> {
        let base_dir = base_dir.as_ref().to_path_buf();
        fs::create_dir_all(&base_dir)?;
        Ok(Self { base_dir })
    }

    pub fn object_path(&self, bucket: &str, object_id: &str) -> PathBuf {
        self.base_dir.join(bucket).join(object_id)
    }

    pub async fn put_object(
        &self,
        bucket: &str,
        object_id: &str,
        data: &[u8],
    ) -> Result<(), StorageError> {
        let bucket_dir = self.base_dir.join(bucket);
        tokio::fs::create_dir_all(&bucket_dir).await?;

        let object_path = self.object_path(bucket, object_id);
        tokio::fs::write(&object_path, data).await?;

        Ok(())
    }

    pub async fn get_object(&self, bucket: &str, object_id: &str) -> Result<Vec<u8>, StorageError> {
        let object_path = self.object_path(bucket, object_id);

        match tokio::fs::read(&object_path).await {
            Ok(data) => Ok(data),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(StorageError::NotFound),
            Err(e) => Err(StorageError::Io(e)),
        }
    }

    pub async fn delete_object(&self, bucket: &str, object_id: &str) -> Result<(), StorageError> {
        let object_path = self.object_path(bucket, object_id);

        match tokio::fs::remove_file(&object_path).await {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == io::ErrorKind::NotFound => Err(StorageError::NotFound),
            Err(e) => Err(StorageError::Io(e)),
        }
    }
}
