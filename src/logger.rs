use crab_vault::logger::{json::JsonLogger, pretty::PrettyLogger};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::app_config::logger::LoggerConfig;

pub fn init(config: LoggerConfig) {
    let logger = tracing_subscriber::registry().with(
        PrettyLogger::new(config.level)
            .with_ansi(config.with_ansi)
            .with_file(config.with_file)
            .with_target(config.with_target)
            .with_thread(config.with_thread),
    );

    if config.dump_path.is_some() {
        let json = JsonLogger::new(config.dump_path.clone().unwrap(), config.dump_level);

        match json {
            Ok(json) => {
                logger
                    .with(
                        json.with_file(config.with_file)
                            .with_target(config.with_target)
                            .with_thread(config.with_thread),
                    )
                    .init();
            }
            Err(e) => {
                logger.init();
                tracing::error!("Cannot open the logger file! Details: {}", e);
            }
        }
    } else {
        logger.init();
    }
}
