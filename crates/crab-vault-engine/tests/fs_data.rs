use crab_vault_engine::{DataEngine, fs::*};
use crab_vault_engine::error::EngineError;
use std::path::PathBuf;

const TEST_DATA_BASE_DIR: &str = "./data_test";

async fn setup(test_name: &str) -> (FsDataEngine, PathBuf) {
    let base_dir = PathBuf::from(TEST_DATA_BASE_DIR).join(test_name);

    if base_dir.exists() {
        tokio::fs::remove_dir_all(&base_dir).await.unwrap();
    }

    let storage = FsDataEngine::new(&base_dir).expect("无法创建根文件夹");

    (storage, base_dir)
}

#[tokio::test]
async fn test_new_creates_base_directory() {
    let test_name = "new_creates_dir";
    let base_dir = PathBuf::from(TEST_DATA_BASE_DIR).join(test_name);

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

    let read_data1 = storage.read_object(bucket_name, object_name).await.unwrap();
    assert_eq!(read_data1, initial_data);

    storage
        .create_object(bucket_name, object_name, new_data)
        .await
        .unwrap();

    let read_data2 = storage.read_object(bucket_name, object_name).await.unwrap();
    assert_eq!(read_data2, new_data);
}