use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

use crate::app_config;

pub fn init() {
    tracing_subscriber::registry()
        .with(EnvFilter::new(app_config::CONFIG.log_level()))
        .with(
            tracing_subscriber::fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_ansi(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_names(true)
                .with_thread_ids(false)
                .pretty(),
        )
        .init();
}
