use std::sync::Arc;

use axum::{Router, routing::MethodRouter};

use crate::{
    http::middleware::auth::AuthLayer,
    storage::{DataSource, MetaSource},
};

mod handler;
mod util;
mod response;

#[derive(Clone)]
pub struct ApiState {
    data_src: Arc<DataSource>,
    meta_src: Arc<MetaSource>,
}

impl ApiState {
    pub fn new(data_src: DataSource, meta_src: MetaSource) -> Self {
        Self {
            data_src: Arc::new(data_src),
            meta_src: Arc::new(meta_src),
        }
    }
}

pub async fn build_router() -> Router<ApiState> {
    use self::handler::*;

    let object_router = MethodRouter::new()
        .put(upload_object)
        .get(get_object)
        .head(head_object)
        .patch(patch_object_meta)
        .delete(delete_object);

    let bucket_router = MethodRouter::new()
        .put(create_bucket)
        .patch(patch_bucket_meta)
        .delete(delete_bucket)
        .get(list_objects_meta)
        .head(head_bucket);

    Router::new()
        .route("/", axum::routing::get(list_buckets_meta))
        .route("/{bucket_name}", bucket_router)
        .route("/{bucket_name}/{*object_name}", object_router)
        .layer(AuthLayer::new())
}
