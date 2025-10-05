use tracing::Level;

use crate::{FdLayer, FdSubscriber};
use std::{collections::HashSet, fmt};

#[derive(Debug, Clone)]
pub enum TracingError {
    AlreadyInitialized,
    InitializationFailed(String),
    InvalidConfiguration(String),
}

impl fmt::Display for TracingError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TracingError::AlreadyInitialized => {
                write!(f, "Global tracing subscriber has already been initialized")
            }
            TracingError::InitializationFailed(msg) => {
                write!(f, "Tracing initialization failed: {}", msg)
            }
            TracingError::InvalidConfiguration(msg) => {
                write!(f, "Invalid tracing configuration: {}", msg)
            }
        }
    }
}

impl std::error::Error for TracingError {}

#[derive(Debug, Clone)]
pub struct FdTracingBuilder {
    max_level: Option<Level>,
    enable_spans: bool,
    enable_opentelemetry: bool,
    structured_logging: bool,
    include_location: bool,
    include_thread_info: bool,
    custom_fields: Vec<String>,
}

impl Default for FdTracingBuilder {
    fn default() -> Self {
        Self {
            max_level: Some(Level::INFO),
            enable_spans: true,
            enable_opentelemetry: false,
            structured_logging: true,
            include_location: true,
            include_thread_info: true,
            custom_fields: Vec::new(),
        }
    }
}

impl FdTracingBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_max_level(mut self, level: Level) -> Self {
        self.max_level = Some(level);
        self
    }

    pub fn with_no_level_filter(mut self) -> Self {
        self.max_level = None;
        self
    }

    pub fn with_spans(mut self, enable: bool) -> Self {
        self.enable_spans = enable;
        self
    }

    pub fn with_opentelemetry(mut self, enable: bool) -> Self {
        self.enable_opentelemetry = enable;
        self
    }

    pub fn with_structured_logging(mut self, enable: bool) -> Self {
        self.structured_logging = enable;
        self
    }

    pub fn with_location(mut self, enable: bool) -> Self {
        self.include_location = enable;
        self
    }

    pub fn with_thread_info(mut self, enable: bool) -> Self {
        self.include_thread_info = enable;
        self
    }

    pub fn with_custom_field<S: Into<String>>(mut self, field: S) -> Self {
        self.custom_fields.push(field.into());
        self
    }

    pub fn with_custom_fields<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.custom_fields
            .extend(fields.into_iter().map(|s| s.into()));
        self
    }

    pub fn build(self) -> FdSubscriber {
        let layer = FdLayer::builder()
            .with_spans(self.enable_spans)
            .with_structured_logging(self.structured_logging)
            .with_location(self.include_location)
            .with_thread_info(self.include_thread_info)
            .with_custom_fields(self.custom_fields)
            .build();

        if let Some(level) = self.max_level {
            FdSubscriber::with_max_level_filter(layer, level)
        } else {
            FdSubscriber::new(layer)
        }
    }

    pub fn init(self) -> Result<(), TracingError> {
        let subscriber = self.build();
        crate::init_global_subscriber(subscriber)
    }

    pub fn try_init(self) -> Result<(), TracingError> {
        let subscriber = self.build();
        crate::try_init_global_subscriber(subscriber)
    }

    pub fn validate(&self) -> Result<(), TracingError> {
        let mut seen_fields = HashSet::new();
        for field in &self.custom_fields {
            if !seen_fields.insert(field) {
                return Err(TracingError::InvalidConfiguration(format!(
                    "Duplicate custom field: {}",
                    field
                )));
            }
        }

        let reserved_fields = &["message", "level", "target", "timestamp", "file", "line"];
        for field in &self.custom_fields {
            if reserved_fields.contains(&field.as_str()) {
                return Err(TracingError::InvalidConfiguration(format!(
                    "Custom field '{}' conflicts with reserved field name",
                    field
                )));
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use tracing::Level;

    use super::*;

    #[test]
    fn test_builder_defaults() {
        let builder = FdTracingBuilder::default();
        assert_eq!(builder.max_level, Some(Level::INFO));
        assert!(builder.enable_spans);
        assert!(!builder.enable_opentelemetry);
        assert!(builder.structured_logging);
        assert!(builder.include_location);
        assert!(builder.include_thread_info);
        assert!(builder.custom_fields.is_empty());
    }

    #[test]
    fn test_builder_configuration() {
        let builder = FdTracingBuilder::new()
            .with_max_level(Level::DEBUG)
            .with_spans(false)
            .with_opentelemetry(true)
            .with_structured_logging(false)
            .with_location(false)
            .with_thread_info(false)
            .with_custom_field("request_id")
            .with_custom_fields(vec!["user_id", "session_id"]);

        assert_eq!(builder.max_level, Some(Level::DEBUG));
        assert!(!builder.enable_spans);
        assert!(builder.enable_opentelemetry);
        assert!(!builder.structured_logging);
        assert!(!builder.include_location);
        assert!(!builder.include_thread_info);
        assert_eq!(builder.custom_fields.len(), 3);
        assert!(builder.custom_fields.contains(&"request_id".to_string()));
        assert!(builder.custom_fields.contains(&"user_id".to_string()));
        assert!(builder.custom_fields.contains(&"session_id".to_string()));
    }

    #[test]
    fn test_validation_success() {
        let builder =
            FdTracingBuilder::new().with_custom_fields(vec!["request_id", "user_id", "session_id"]);

        assert!(builder.validate().is_ok());
    }

    #[test]
    fn test_validation_duplicate_fields() {
        let builder = FdTracingBuilder::new()
            .with_custom_field("request_id")
            .with_custom_field("request_id");

        match builder.validate() {
            Err(TracingError::InvalidConfiguration(msg)) => {
                assert!(msg.contains("Duplicate custom field"));
            }
            _ => panic!("Expected InvalidConfiguration error"),
        }
    }

    #[test]
    fn test_validation_reserved_fields() {
        let builder = FdTracingBuilder::new().with_custom_field("message");

        match builder.validate() {
            Err(TracingError::InvalidConfiguration(msg)) => {
                assert!(msg.contains("conflicts with reserved field name"));
            }
            _ => panic!("Expected InvalidConfiguration error"),
        }
    }

    #[test]
    fn test_no_level_filter() {
        let builder = FdTracingBuilder::new()
            .with_max_level(Level::ERROR)
            .with_no_level_filter();

        assert_eq!(builder.max_level, None);
    }
}
