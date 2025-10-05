use fd_log::{LogLevel, SystemLogger};
use opentelemetry::{logs::Severity, KeyValue};
use opentelemetry_sdk::Resource;

pub fn severity_to_fd_level(severity: Severity) -> LogLevel {
    match severity {
        Severity::Trace | Severity::Trace2 | Severity::Trace3 | Severity::Trace4 => LogLevel::Debug,
        Severity::Debug | Severity::Debug2 | Severity::Debug3 | Severity::Debug4 => LogLevel::Debug,
        Severity::Info | Severity::Info2 | Severity::Info3 | Severity::Info4 => LogLevel::Info,
        Severity::Warn | Severity::Warn2 | Severity::Warn3 | Severity::Warn4 => LogLevel::Warning,
        Severity::Error | Severity::Error2 | Severity::Error3 | Severity::Error4 => LogLevel::Error,
        Severity::Fatal | Severity::Fatal2 | Severity::Fatal3 | Severity::Fatal4 => {
            LogLevel::Critical
        }
    }
}

pub fn create_resource() -> Resource {
    let mut attributes = Vec::new();
    attributes.push(KeyValue::new("service.name", SystemLogger::app()));
    attributes.push(KeyValue::new(
        "service.instance.id",
        format!("{}", SystemLogger::app_id()),
    ));

    attributes.push(KeyValue::new("host.name", SystemLogger::host()));
    attributes.push(KeyValue::new(
        "host.id",
        format!("{}", SystemLogger::host_id()),
    ));

    attributes.push(KeyValue::new(
        "process.pid",
        format!("{}", SystemLogger::tid()),
    ));
    attributes.push(KeyValue::new(
        "process.executable.name",
        SystemLogger::app(),
    ));

    Resource::new(attributes)
}

pub fn log_to_fd(
    level: Severity,
    message: &str,
    attributes: &[KeyValue],
    file: Option<&str>,
    line: Option<u32>,
) {
    let fd_level = severity_to_fd_level(level);
    let mut formatted_message = message.to_string();

    if !attributes.is_empty() {
        let mut attr_strs = Vec::new();
        for kv in attributes {
            attr_strs.push(format!("{} = {:?}", kv.key, kv.value));
        }

        if !formatted_message.is_empty() {
            formatted_message.push_str(" { ");
        } else {
            formatted_message.push_str("{ ");
        }
        formatted_message.push_str(&attr_strs.join(", "));
        formatted_message.push_str(" }");
    }

    let file_str = file.unwrap_or("opentelemetry");
    let line_num = line.unwrap_or(0);

    fd_log::_fd_log(
        fd_level,
        file_str,
        line_num,
        "opentelemetry",
        &formatted_message,
    );
}

/// A simple OpenTelemetry-compatible log exporter that forwards to fd_log
#[derive(Debug, Clone)]
pub struct FdLogExporter {
    include_trace_context: bool,
}

impl FdLogExporter {
    pub fn new() -> Self {
        Self {
            include_trace_context: true,
        }
    }

    pub fn with_trace_context(mut self, include: bool) -> Self {
        self.include_trace_context = include;
        self
    }

    pub fn log(&self, level: Severity, message: &str, attributes: &[KeyValue]) {
        log_to_fd(level, message, attributes, None, None);
    }

    pub fn flush(&self) {
        SystemLogger::flush();
    }
}

impl Default for FdLogExporter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_severity_mapping() {
        assert_eq!(severity_to_fd_level(Severity::Trace), LogLevel::Debug);
        assert_eq!(severity_to_fd_level(Severity::Debug), LogLevel::Debug);
        assert_eq!(severity_to_fd_level(Severity::Info), LogLevel::Info);
        assert_eq!(severity_to_fd_level(Severity::Warn), LogLevel::Warning);
        assert_eq!(severity_to_fd_level(Severity::Error), LogLevel::Error);
        assert_eq!(severity_to_fd_level(Severity::Fatal), LogLevel::Critical);

        assert_eq!(severity_to_fd_level(Severity::Debug2), LogLevel::Debug);
        assert_eq!(severity_to_fd_level(Severity::Info3), LogLevel::Info);
        assert_eq!(severity_to_fd_level(Severity::Error4), LogLevel::Error);
    }

    #[test]
    fn test_create_resource() {
        let resource = create_resource();
        assert!(!resource.is_empty());

        let attrs: Vec<_> = resource.iter().collect();
        let has_service_name = attrs.iter().any(|(key, _)| key.as_str() == "service.name");
        let has_host_name = attrs.iter().any(|(key, _)| key.as_str() == "host.name");

        assert!(has_service_name);
        assert!(has_host_name);
    }

    #[test]
    fn test_log_to_fd() {
        let attributes = vec![
            KeyValue::new("request_id", "12345"),
            KeyValue::new("user_id", 42),
        ];

        log_to_fd(
            Severity::Info,
            "Test message",
            &attributes,
            Some("test.rs"),
            Some(123),
        );
    }

    #[test]
    fn test_exporter() {
        let exporter = FdLogExporter::new();
        assert!(exporter.include_trace_context);

        let attributes = vec![KeyValue::new("test", "value")];
        exporter.log(Severity::Info, "Test log", &attributes);
        exporter.flush();
    }
}
