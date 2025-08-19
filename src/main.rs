mod api;
mod app_config;
mod common;
mod logger;
mod storage;

use axum::extract::Request;
use base64::Engine;
use base64::prelude::BASE64_STANDARD_NO_PAD;
use std::{net::Ipv4Addr, time::Duration};
use storage::DataSource;
use tower_http::trace::DefaultOnRequest;
use tower_http::trace::DefaultOnResponse;
use tower_http::{
    cors::{self, CorsLayer},
    trace::TraceLayer,
};

use crate::{
    api::AppState,
    storage::{DataStorage, MetaSource, MetaStorage},
};

#[tokio::main]
async fn main() {
    let conf_ref = &app_config::CONFIG;
    logger::init();

    let data_src = DataSource::new(conf_ref.data_source()).expect("Failed to create data storage");
    let meta_src = MetaSource::new(conf_ref.meta_source()).expect("Failed to create data storage");
    let state = AppState::new(data_src, meta_src);

    let tracing_layer = TraceLayer::new_for_http()
        .make_span_with(|req: &Request| {
            let method = req.method().to_string();
            let uri = req.uri().to_string();
            let request_id = BASE64_STANDARD_NO_PAD.encode(uuid::Uuid::new_v4()); // 使用 base64 编码的 uuid 作为请求 req_id
            tracing::info_span!("", request_id, uri, method)
        })
        .on_failure(())
        .on_request(DefaultOnRequest::new().level(tracing::Level::INFO))
        .on_response(DefaultOnResponse::new().level(tracing::Level::INFO));

    let cors_layer = CorsLayer::new()
        .allow_methods(cors::Any)
        .allow_headers(cors::Any)
        .allow_origin(cors::Any)
        .allow_credentials(false)
        .max_age(Duration::from_secs(3600 * 24));

    let app = api::build_router()
        .layer(cors_layer)
        .layer(tracing_layer)
        .with_state(state);

    let listener =
        tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, app_config::CONFIG.port()))
            .await
            .unwrap();

    tracing::info!(
        "Server running on http://{}",
        listener.local_addr().unwrap()
    );

    axum::serve(listener, app.into_make_service())
        .await
        .unwrap();
}
