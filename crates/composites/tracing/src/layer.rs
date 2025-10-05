use crate::{tracing_to_fd_level, SpanTracker};
use fd_json::JsonValue;
use fd_log::SystemLogger;
use std::collections::HashMap;
use std::fmt::Write;
use std::sync::Arc;
use tracing_core::{
    field::{Field, Visit},
    span::{Attributes, Id, Record},
    Event, Interest, Metadata, Subscriber,
};
use tracing_subscriber::Layer;

#[derive(Debug)]
pub struct FdLayer {
    span_tracker: Arc<SpanTracker>,
    config: LayerConfig,
}

#[derive(Debug, Clone)]
pub struct LayerConfig {
    pub enable_spans: bool,
    pub structured_logging: bool,
    pub include_location: bool,
    pub include_thread_info: bool,
    pub custom_fields: Vec<String>,
}

impl Default for LayerConfig {
    fn default() -> Self {
        Self {
            enable_spans: true,
            structured_logging: true,
            include_location: true,
            include_thread_info: true,
            custom_fields: Vec::new(),
        }
    }
}

impl FdLayer {
    pub fn new() -> Self {
        Self {
            span_tracker: Arc::new(SpanTracker::new()),
            config: LayerConfig::default(),
        }
    }

    pub fn builder() -> FdLayerBuilder {
        FdLayerBuilder::new()
    }

    pub fn span_tracker(&self) -> Arc<SpanTracker> {
        Arc::clone(&self.span_tracker)
    }
}

impl Default for FdLayer {
    fn default() -> Self {
        Self::new()
    }
}

impl<S> Layer<S> for FdLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        let metadata = event.metadata();
        let level = tracing_to_fd_level(metadata.level());

        let mut visitor = FieldVisitor::new(&self.config);
        event.record(&mut visitor);

        let message = self.format_message(metadata, &visitor);
        let (file, line) = if self.config.include_location {
            (
                metadata.file().unwrap_or("unknown"),
                metadata.line().unwrap_or(0),
            )
        } else {
            ("", 0)
        };

        let target = metadata.target();

        fd_log::_fd_log(level, file, line, target, &message);
    }

    fn on_new_span(
        &self,
        attrs: &Attributes<'_>,
        id: &Id,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if !self.config.enable_spans {
            return;
        }

        let metadata = attrs.metadata();
        let mut visitor = FieldVisitor::new(&self.config);
        attrs.record(&mut visitor);

        let span_data = crate::span::SpanData {
            id: id.into_u64(),
            name: metadata.name().to_string(),
            target: metadata.target().to_string(),
            level: *metadata.level(),
            fields: visitor.fields,
            file: metadata.file().map(|s| s.to_string()),
            line: metadata.line(),
        };

        self.span_tracker.on_new_span(span_data);
    }

    fn on_enter(&self, id: &Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if self.config.enable_spans {
            self.span_tracker.on_enter(id.into_u64());
        }
    }

    fn on_exit(&self, id: &Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if self.config.enable_spans {
            self.span_tracker.on_exit(id.into_u64());
        }
    }

    fn on_close(&self, id: Id, _ctx: tracing_subscriber::layer::Context<'_, S>) {
        if self.config.enable_spans {
            self.span_tracker.on_close(id.into_u64());
        }
    }

    fn on_record(
        &self,
        id: &Id,
        values: &Record<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        if !self.config.enable_spans {
            return;
        }

        let mut visitor = FieldVisitor::new(&self.config);
        values.record(&mut visitor);

        self.span_tracker.on_record(id.into_u64(), visitor.fields);
    }

    fn register_callsite(&self, _metadata: &'static Metadata<'static>) -> Interest {
        Interest::always()
    }

    fn enabled(
        &self,
        metadata: &Metadata<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) -> bool {
        let fd_level = tracing_to_fd_level(metadata.level());
        fd_level >= SystemLogger::level_stderr() || fd_level >= SystemLogger::level_logfile()
    }
}

impl FdLayer {
    fn format_message(&self, metadata: &Metadata<'_>, visitor: &FieldVisitor) -> String {
        let mut message = String::new();

        if let Some(msg) = visitor.fields.get("message") {
            message.push_str(&format_field_value(msg));
        }

        if self.config.structured_logging && visitor.fields.len() > 1 {
            let mut structured_fields = Vec::new();

            for (key, value) in &visitor.fields {
                if key != "message" {
                    structured_fields.push(format!("{} = {}", key, format_field_value(value)));
                }
            }

            if !structured_fields.is_empty() {
                if !message.is_empty() {
                    message.push_str(" { ");
                } else {
                    message.push_str("{ ");
                }
                message.push_str(&structured_fields.join(", "));
                message.push_str(" }");
            }
        }

        if self.config.enable_spans {
            if let Some(span_context) = self.span_tracker.current_context() {
                if !message.is_empty() {
                    message.push_str(" ");
                }
                write!(&mut message, "[span: {}]", span_context.name).unwrap_or(());
            }
        }

        if self.config.include_thread_info {
            let current_thread = std::thread::current();
            let thread_name = current_thread.name().unwrap_or("unnamed");
            let thread_id = SystemLogger::tid();
            if !message.is_empty() {
                message.push_str(" ");
            }
            write!(&mut message, "[thread: {} ({})]", thread_name, thread_id).unwrap_or(());
        }

        for field_name in &self.config.custom_fields {
            if let Some(value) = visitor.fields.get(field_name) {
                if !message.is_empty() {
                    message.push_str(" ");
                }
                write!(
                    &mut message,
                    "[{}: {}]",
                    field_name,
                    format_field_value(value)
                )
                .unwrap_or(());
            }
        }

        if message.is_empty() {
            format!("Event at {}", metadata.target())
        } else {
            message
        }
    }
}

fn format_field_value(value: &JsonValue) -> String {
    match value {
        JsonValue::String(s) => s.clone(),
        JsonValue::Number(n) => n.to_string(),
        JsonValue::Bool(b) => b.to_string(),
        JsonValue::Null => "null".to_string(),
        JsonValue::Array(_) | JsonValue::Object(_) => {
            fd_json::to_string_compact(value).unwrap_or_else(|_| "invalid_json".to_string())
        }
    }
}

#[derive(Debug)]
pub struct FieldVisitor {
    pub fields: HashMap<String, JsonValue>,
    config: LayerConfig,
}

impl FieldVisitor {
    pub fn new(config: &LayerConfig) -> Self {
        Self {
            fields: HashMap::new(),
            config: config.clone(),
        }
    }
}

impl Visit for FieldVisitor {
    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        self.fields.insert(
            field.name().to_string(),
            JsonValue::String(format!("{:?}", value)),
        );
    }

    fn record_str(&mut self, field: &Field, value: &str) {
        self.fields.insert(
            field.name().to_string(),
            JsonValue::String(value.to_string()),
        );
    }

    fn record_bool(&mut self, field: &Field, value: bool) {
        self.fields
            .insert(field.name().to_string(), JsonValue::Bool(value));
    }

    fn record_i64(&mut self, field: &Field, value: i64) {
        self.fields
            .insert(field.name().to_string(), JsonValue::Number(value as f64));
    }

    fn record_u64(&mut self, field: &Field, value: u64) {
        self.fields
            .insert(field.name().to_string(), JsonValue::Number(value as f64));
    }

    fn record_f64(&mut self, field: &Field, value: f64) {
        self.fields
            .insert(field.name().to_string(), JsonValue::Number(value));
    }
}

#[derive(Debug, Clone)]
pub struct FdLayerBuilder {
    config: LayerConfig,
}

impl FdLayerBuilder {
    pub fn new() -> Self {
        Self {
            config: LayerConfig::default(),
        }
    }

    pub fn with_spans(mut self, enable: bool) -> Self {
        self.config.enable_spans = enable;
        self
    }

    pub fn with_structured_logging(mut self, enable: bool) -> Self {
        self.config.structured_logging = enable;
        self
    }

    pub fn with_location(mut self, enable: bool) -> Self {
        self.config.include_location = enable;
        self
    }

    pub fn with_thread_info(mut self, enable: bool) -> Self {
        self.config.include_thread_info = enable;
        self
    }

    pub fn with_custom_field<S: Into<String>>(mut self, field: S) -> Self {
        self.config.custom_fields.push(field.into());
        self
    }

    pub fn with_custom_fields<I, S>(mut self, fields: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.config
            .custom_fields
            .extend(fields.into_iter().map(|s| s.into()));
        self
    }

    pub fn build(self) -> FdLayer {
        FdLayer {
            span_tracker: Arc::new(SpanTracker::new()),
            config: self.config,
        }
    }
}

impl Default for FdLayerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fd_json::json;

    #[test]
    fn test_field_visitor_creation() {
        let config = LayerConfig::default();
        let visitor = FieldVisitor::new(&config);

        assert!(visitor.fields.is_empty());
    }

    #[test]
    fn test_format_field_value() {
        assert_eq!(format_field_value(&json!("hello")), "hello");
        assert_eq!(format_field_value(&json!(42)), "42");
        assert_eq!(format_field_value(&json!(true)), "true");
        assert_eq!(format_field_value(&json!(null)), "null");
        assert_eq!(
            format_field_value(&json!({"key": "value"})),
            r#"{"key":"value"}"#
        );
    }

    #[test]
    fn test_layer_builder() {
        let layer = FdLayer::builder()
            .with_spans(false)
            .with_structured_logging(false)
            .with_location(false)
            .with_thread_info(false)
            .with_custom_field("request_id")
            .build();

        assert!(!layer.config.enable_spans);
        assert!(!layer.config.structured_logging);
        assert!(!layer.config.include_location);
        assert!(!layer.config.include_thread_info);
        assert_eq!(layer.config.custom_fields, vec!["request_id"]);
    }
}
