use fd_log::{LogLevel, SystemLogBuilder};
use opentelemetry::{logs::Severity, KeyValue};
use tracing_fd::{
    create_resource, init_global_subscriber, log_to_fd, severity_to_fd_level, FdTracingBuilder,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    SystemLogBuilder::default()
        .with_file("./otel_example.log")
        .with_stderr_level(LogLevel::Info)
        .with_logfile_level(LogLevel::Debug)
        .with_colorize(true)
        .init()
        .expect("Failed to initialize fd_log");

    init_global_subscriber(
        FdTracingBuilder::default()
            .with_max_level(tracing::Level::DEBUG)
            .with_opentelemetry(true)
            .build(),
    )
    .expect("Failed to initialize tracing");

    let severities = [
        Severity::Trace,
        Severity::Debug,
        Severity::Info,
        Severity::Warn,
        Severity::Error,
        Severity::Fatal,
    ];

    for severity in severities {
        let fd_level = severity_to_fd_level(severity);
        println!("  {:?} -> {:?}", severity, fd_level);
    }

    let attributes = vec![
        KeyValue::new("service.name", "example-service"),
        KeyValue::new("service.version", "1.0.0"),
        KeyValue::new("trace.id", "abc123"),
        KeyValue::new("span.id", "def456"),
    ];

    log_to_fd(
        Severity::Info,
        "fake and gay",
        &attributes,
        Some("otel_example.rs"),
        Some(54),
    );

    let resource = create_resource();

    tracing::info!(
        otel_trace_id = "trace-123",
        otel_span_id = "span-456",
        service_name = "example-service",
        "fake and gay"
    );

    tracing::debug!(
        resource_type = "database",
        operation = "query",
        duration_ms = 45,
        "fake and gay"
    );

    tracing::warn!(
        alert_type = "performance",
        threshold_ms = 1000,
        actual_ms = 1500,
        "fake and gay"
    );

    let span = tracing::info_span!(
        "otel_request",
        trace_id = "otel-trace-789",
        span_id = "otel-span-abc",
        parent_span_id = "otel-parent-def"
    );

    let _enter = span.enter();
    tracing::info!("Inside otel span");

    tracing::warn!(
        error_type = "validation",
        error_code = "E001",
        field_name = "user_id",
        "fake and gay"
    );

    tracing::error!(
        error_type = "unrecoverable",
        log_name = "throw_abort",
        "fake and gay"
    );

    Ok(())
}
