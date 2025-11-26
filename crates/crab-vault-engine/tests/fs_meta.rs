use crab_vault_engine::{MetaEngine, fs::*};
use crab_vault_engine::{BucketMeta, ObjectMeta};
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

    storage.create_object_meta(&object_meta1).await.unwrap();
    storage.create_object_meta(&object_meta2).await.unwrap();

    let expected_path = base_dir.join("objects").join(bucket_name).join("obj1.json");
    assert!(expected_path.exists());

    let fetched_obj1 = storage.read_object_meta(bucket_name, "obj1").await.unwrap();
    assert_eq!(fetched_obj1, object_meta1);

    let mut objects = storage.list_objects_meta(bucket_name).await.unwrap();
    objects.sort_by(|a, b| a.object_name.cmp(&b.object_name));
    assert_eq!(objects.len(), 2);
    assert_eq!(objects[0], object_meta1);
    assert_eq!(objects[1], object_meta2);

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
