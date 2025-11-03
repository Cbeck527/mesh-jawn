use std::io::{self, IsTerminal};
use tracing_subscriber::fmt::format::FmtSpan;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogFormat {
    Console,
    Json,
}

pub fn configure_logging(verbose: bool, log_format: Option<LogFormat>) {
    let use_json = if let Some(LogFormat::Json) = log_format {
        true
    } else {
        !io::stderr().is_terminal()
    };

    let level = if verbose { "debug" } else { "info" };

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("{},meshtastic=info", level)));

    if use_json {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .json()
                    .with_current_span(true)
                    .with_span_list(false),
            )
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(
                fmt::layer()
                    .with_target(false)
                    .with_thread_ids(false)
                    .with_thread_names(false)
                    .with_span_events(FmtSpan::NONE),
            )
            .init();
    }
}
