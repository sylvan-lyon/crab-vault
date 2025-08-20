use std::sync::Arc;

use axum::{Router, routing::MethodRouter};

use crate::storage::{DataSource, MetaSource};

mod handler;
mod util;

#[derive(Clone)]
pub struct AppState {
    data_src: Arc<DataSource>,
    meta_src: Arc<MetaSource>,
}

impl AppState {
    pub fn new(data_src: DataSource, meta_src: MetaSource) -> Self {
        Self {
            data_src: Arc::new(data_src),
            meta_src: Arc::new(meta_src),
        }
    }
}

pub fn build_router() -> Router<AppState> {
    use self::handler::*;

    // 路由定义，使用您设计的 RESTful 风格
    let object_router = MethodRouter::new()
        .put(upload_object)
        .get(get_object)
        .head(head_object)
        .patch(patch_object_meta)
        .delete(delete_object);

    let bucket_router = MethodRouter::new()
        .put(create_bucket)
        .head(head_bucket)
        .get(list_objects_meta)
        .patch(patch_bucket_meta)
        .delete(delete_bucket);

    Router::new()
        // 暂时省略根路径的 list_buckets
        .route("/", axum::routing::get(list_buckets_meta))
        .route("/{bucket_name}", bucket_router)
        .route("/{bucket_name}/{*object_name}", object_router)
}
