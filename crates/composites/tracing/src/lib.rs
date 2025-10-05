pub mod builder;
pub mod layer;
pub mod otel;
pub mod span;
pub mod subscriber;

pub use builder::{FdTracingBuilder, TracingError};
use fd_log::LogLevel;
pub use layer::FdLayer;
pub use otel::{create_resource, log_to_fd, severity_to_fd_level, FdLogExporter};
pub use span::{SpanContext, SpanData, SpanTracker};
pub use subscriber::FdSubscriber;

use once_cell::sync::OnceCell;
use std::sync::Arc;
use tracing::{Level, Subscriber};
use tracing_core::Dispatch;

static GLOBAL_SUBSCRIBER: OnceCell<Arc<dyn tracing::Subscriber + Send + Sync>> = OnceCell::new();

/// Initialize the global tracing subscriber with the fd_log backend
pub fn init_global_subscriber<S>(subscriber: S) -> Result<(), TracingError>
where
    S: tracing::Subscriber + Send + Sync + 'static,
{
    let subscriber = Arc::new(subscriber);

    GLOBAL_SUBSCRIBER
        .set(subscriber.clone())
        .map_err(|_| TracingError::AlreadyInitialized)?;

    let dispatch = Dispatch::new(subscriber);
    tracing::dispatcher::set_global_default(dispatch)
        .map_err(|e| TracingError::InitializationFailed(e.to_string()))?;

    Ok(())
}

pub fn try_init_global_subscriber<S>(subscriber: S) -> Result<(), TracingError>
where
    S: Subscriber + Send + Sync + 'static,
{
    match init_global_subscriber(subscriber) {
        Ok(()) => Ok(()),
        Err(TracingError::AlreadyInitialized) => Ok(()),
        Err(e) => Err(e),
    }
}

pub fn global_subscriber() -> Option<Arc<dyn Subscriber + Send + Sync>> {
    GLOBAL_SUBSCRIBER.get().cloned()
}

pub fn tracing_to_fd_level(level: &Level) -> LogLevel {
    match *level {
        Level::TRACE => LogLevel::Debug,
        Level::DEBUG => LogLevel::Debug,
        Level::INFO => LogLevel::Info,
        Level::WARN => LogLevel::Warning,
        Level::ERROR => LogLevel::Error,
    }
}

pub fn fd_to_tracing_level(level: LogLevel) -> Option<Level> {
    match level {
        LogLevel::Debug => Some(Level::DEBUG),
        LogLevel::Info => Some(Level::INFO),
        LogLevel::Notice => Some(Level::INFO),
        LogLevel::Warning => Some(Level::WARN),
        LogLevel::Error => Some(Level::ERROR),
        LogLevel::Critical => Some(Level::ERROR),
        LogLevel::Alert => Some(Level::ERROR),
        LogLevel::Emergency => Some(Level::ERROR),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subscriber_lifecycle() {
        let subscriber = FdTracingBuilder::default().build();
        match init_global_subscriber(subscriber) {
            Ok(()) => {
                let subscriber2 = FdTracingBuilder::default().build();
                assert!(matches!(
                    init_global_subscriber(subscriber2),
                    Err(TracingError::AlreadyInitialized)
                ));
            }
            Err(TracingError::AlreadyInitialized) => {}
            Err(e) => panic!("Unexpected error: {:?}", e),
        }
    }
}
