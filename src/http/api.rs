use std::sync::Arc;

use axum::{routing::MethodRouter, Router};

use crate::http::middleware::auth::AuthLayer;

use crab_vault::engine::{DataSource, MetaSource};

mod handler;
mod response;
mod util;

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

    let health = MethodRouter::new()
        .get(health)
        .head(health);

    Router::new()
        .route("/", axum::routing::get(list_buckets_meta))
        .route("/{bucket_name}", bucket_router)
        .route("/{bucket_name}/{*object_name}", object_router)
        .layer(AuthLayer::new())
        .route("/health", health)
}
