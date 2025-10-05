use crate::FdLayer;
use tracing_core::{Interest, Metadata};
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt, Layer, Registry};

pub struct FdSubscriber {
    inner: Box<dyn tracing::Subscriber + Send + Sync>,
}

impl FdSubscriber {
    pub fn new(layer: FdLayer) -> Self {
        let subscriber = Registry::default().with(layer);
        Self {
            inner: Box::new(subscriber),
        }
    }

    pub fn with_max_level_filter(layer: FdLayer, level: tracing::Level) -> Self {
        let filter = LevelFilter::from_level(level);
        let subscriber = Registry::default().with(layer.with_filter(filter));
        Self {
            inner: Box::new(subscriber),
        }
    }
}

impl tracing::Subscriber for FdSubscriber {
    fn enabled(&self, metadata: &Metadata<'_>) -> bool {
        self.inner.enabled(metadata)
    }

    fn new_span(&self, span: &tracing::span::Attributes<'_>) -> tracing::span::Id {
        self.inner.new_span(span)
    }

    fn record(&self, span: &tracing::span::Id, values: &tracing::span::Record<'_>) {
        self.inner.record(span, values)
    }

    fn record_follows_from(&self, span: &tracing::span::Id, follows: &tracing::span::Id) {
        self.inner.record_follows_from(span, follows)
    }

    fn event(&self, event: &tracing::Event<'_>) {
        self.inner.event(event)
    }

    fn enter(&self, span: &tracing::span::Id) {
        self.inner.enter(span)
    }

    fn exit(&self, span: &tracing::span::Id) {
        self.inner.exit(span)
    }

    fn clone_span(&self, id: &tracing::span::Id) -> tracing::span::Id {
        self.inner.clone_span(id)
    }

    fn try_close(&self, id: tracing::span::Id) -> bool {
        self.inner.try_close(id)
    }

    fn register_callsite(&self, metadata: &'static Metadata<'static>) -> Interest {
        self.inner.register_callsite(metadata)
    }

    fn max_level_hint(&self) -> Option<tracing_core::LevelFilter> {
        self.inner.max_level_hint()
    }
}

pub fn init() -> Result<(), crate::TracingError> {
    let layer = FdLayer::new();
    let subscriber = FdSubscriber::new(layer);
    crate::init_global_subscriber(subscriber)
}

pub fn try_init() -> Result<(), crate::TracingError> {
    let layer = FdLayer::new();
    let subscriber = FdSubscriber::new(layer);
    crate::try_init_global_subscriber(subscriber)
}

pub fn init_with_level(level: tracing::Level) -> Result<(), crate::TracingError> {
    let layer = FdLayer::new();
    let subscriber = FdSubscriber::with_max_level_filter(layer, level);
    crate::init_global_subscriber(subscriber)
}

pub fn try_init_with_level(level: tracing::Level) -> Result<(), crate::TracingError> {
    let layer = FdLayer::new();
    let subscriber = FdSubscriber::with_max_level_filter(layer, level);
    crate::try_init_global_subscriber(subscriber)
}
