use std::sync::Arc;

use axum::Router;

use crate::storage::{DataSource, MetaSource};

mod data;
mod meta;

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
    Router::new()
        .nest("/data", data::build_router())
        .nest("/meta", meta::build_router())
}
