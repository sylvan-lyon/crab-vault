use std::{net::Ipv4Addr, time::Duration};

use axum::extract::Request;
use base64::{Engine, prelude::BASE64_STANDARD};
use crab_vault::engine::{DataEngine, DataSource, MetaEngine, MetaSource};
use tower_http::{
    cors::{self, CorsLayer},
    normalize_path::NormalizePathLayer,
    trace::{DefaultOnRequest, DefaultOnResponse, TraceLayer},
};

use crate::{
    app_config::{self, ConfigItem},
    cli::run::RunArgs,
    http::api::{self, ApiState},
    logger,
};

pub async fn run(config_path: String, args: RunArgs) {
    let config = app_config::StaticAppConfig::from_file(config_path)
        .merge_cli(args)
        .into_runtime()
        .map_err(|e| e.exit_now())
        .unwrap();

    logger::init(config.logger);

    let data_src = DataSource::new(&config.data.source).expect("Failed to create data storage");
    let meta_src = MetaSource::new(&config.meta.source).expect("Failed to create meta storage");
    let state = ApiState::new(data_src, meta_src);

    let tracing_layer = TraceLayer::new_for_http()
        .make_span_with(|req: &Request| {
            let method = req.method().to_string();
            let uri = req.uri().to_string();
            let req_id = BASE64_STANDARD.encode(uuid::Uuid::new_v4()); // 使用 base64 编码的 uuid 作为请求 req_id
            tracing::info_span!("[request]", req_id, method, uri)
        })
        .on_failure(())
        .on_request(DefaultOnRequest::new().level(tracing::Level::INFO))
        .on_response(DefaultOnResponse::new().level(tracing::Level::INFO));

    let normalize_path_layer = NormalizePathLayer::trim_trailing_slash();

    let cors_layer = CorsLayer::new()
        .allow_methods(cors::Any)
        .allow_headers(cors::Any)
        .allow_origin(cors::Any)
        .allow_credentials(false)
        .max_age(Duration::from_secs(3600 * 24));

    let app = api::build_router(
        config.auth.jwt_decoder_config.decoder,
        config.auth.path_rules,
    )
    .await
    .layer(cors_layer)
    .layer(tracing_layer)
    .layer(normalize_path_layer)
    .with_state(state);

    let listener = tokio::net::TcpListener::bind((Ipv4Addr::UNSPECIFIED, config.server.port))
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
