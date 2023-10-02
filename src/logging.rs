#[macro_export]
macro_rules! is_debug_level_enabled {
    () => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::enabled!(tracing::Level::DEBUG)
            } else if #[cfg(feature = "logging")] {
                log::log_enabled!(log::Level::Debug)
            } else {
                false
            }
        }
    }};
}

#[macro_export]
macro_rules! is_trace_level_enabled {
    () => {{
        cfg_if::cfg_if! {
            if #[cfg(feature = "tracing")] {
                tracing::enabled!(tracing::Level::TRACE)
            } else if #[cfg(feature = "logging")] {
                log::log_enabled!(log::Level::Trace)
            } else {
                false
            }
        }
    }};
}

#[cfg(test)]
pub fn enable_logging() {
    cfg_if::cfg_if! {
        if #[cfg(feature = "tracing")] {
            let _ = tracing_subscriber::fmt()
                .json()
                .with_max_level(tracing::Level::TRACE)
                .try_init();
        } else if #[cfg(feature = "logging")] {
            let _ = env_logger::builder()
                .filter_level(log::LevelFilter::max())
                .is_test(true)
                .try_init();
        }
    }
}

#[cfg(test)]
pub fn info(text: String) {
    cfg_if::cfg_if! {
        if #[cfg(feature = "tracing")] {
            tracing::info!(text);
        } else if #[cfg(feature = "logging")] {
            log::info!("{}", text);
        }
    }
}
