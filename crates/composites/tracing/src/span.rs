use fd_json::JsonValue;
use parking_lot::RwLock;
use smallvec::SmallVec;
use std::collections::HashMap;
use std::thread::ThreadId;

#[derive(Debug, Clone)]
pub struct SpanData {
    pub id: u64,
    pub name: String,
    pub target: String,
    pub level: tracing::Level,
    pub fields: HashMap<String, JsonValue>,
    pub file: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct SpanContext {
    pub id: u64,
    pub name: String,
    pub target: String,
    pub level: tracing::Level,
    pub fields: HashMap<String, JsonValue>,
    pub depth: usize,
    pub parent_id: Option<u64>,
}

#[derive(Debug, Clone)]
struct SpanStackEntry {
    data: SpanData,
    depth: usize,
    parent_id: Option<u64>,
}

#[derive(Debug)]
pub struct SpanTracker {
    /// maps span ID to span data
    spans: RwLock<HashMap<u64, SpanData>>,
    /// maps thread ID to span stack
    thread_stacks: RwLock<HashMap<ThreadId, SmallVec<[SpanStackEntry; 8]>>>,
}

impl SpanTracker {
    pub fn new() -> Self {
        Self {
            spans: RwLock::new(HashMap::new()),
            thread_stacks: RwLock::new(HashMap::new()),
        }
    }

    pub fn on_new_span(&self, span_data: SpanData) {
        let mut spans = self.spans.write();
        spans.insert(span_data.id, span_data);
    }

    pub fn on_enter(&self, span_id: u64) {
        let thread_id = std::thread::current().id();
        let mut thread_stacks = self.thread_stacks.write();
        let stack = thread_stacks.entry(thread_id).or_insert_with(SmallVec::new);

        if let Some(span_data) = self.spans.read().get(&span_id).cloned() {
            let parent_id = stack.last().map(|entry| entry.data.id);
            let depth = stack.len();

            stack.push(SpanStackEntry {
                data: span_data,
                depth,
                parent_id,
            });
        }
    }

    pub fn on_exit(&self, span_id: u64) {
        let thread_id = std::thread::current().id();
        let mut thread_stacks = self.thread_stacks.write();

        if let Some(stack) = thread_stacks.get_mut(&thread_id) {
            if let Some(current) = stack.last() {
                if current.data.id == span_id {
                    stack.pop();
                }
            }

            if stack.is_empty() {
                thread_stacks.remove(&thread_id);
            }
        }
    }

    pub fn on_close(&self, span_id: u64) {
        let mut spans = self.spans.write();
        spans.remove(&span_id);

        let thread_id = std::thread::current().id();
        let mut thread_stacks = self.thread_stacks.write();

        if let Some(stack) = thread_stacks.get_mut(&thread_id) {
            stack.retain(|entry| entry.data.id != span_id);

            if stack.is_empty() {
                thread_stacks.remove(&thread_id);
            }
        }
    }

    pub fn on_record(&self, span_id: u64, new_fields: HashMap<String, JsonValue>) {
        let mut spans = self.spans.write();
        if let Some(span_data) = spans.get_mut(&span_id) {
            span_data.fields.extend(new_fields);
        }
    }

    pub fn current_context(&self) -> Option<SpanContext> {
        let thread_id = std::thread::current().id();
        let thread_stacks = self.thread_stacks.read();

        if let Some(stack) = thread_stacks.get(&thread_id) {
            if let Some(current) = stack.last() {
                return Some(SpanContext {
                    id: current.data.id,
                    name: current.data.name.clone(),
                    target: current.data.target.clone(),
                    level: current.data.level,
                    fields: current.data.fields.clone(),
                    depth: current.depth,
                    parent_id: current.parent_id,
                });
            }
        }

        None
    }

    pub fn current_stack(&self) -> Vec<SpanContext> {
        let thread_id = std::thread::current().id();
        let thread_stacks = self.thread_stacks.read();

        if let Some(stack) = thread_stacks.get(&thread_id) {
            stack
                .iter()
                .map(|entry| SpanContext {
                    id: entry.data.id,
                    name: entry.data.name.clone(),
                    target: entry.data.target.clone(),
                    level: entry.data.level,
                    fields: entry.data.fields.clone(),
                    depth: entry.depth,
                    parent_id: entry.parent_id,
                })
                .collect()
        } else {
            Vec::new()
        }
    }

    pub fn get_span(&self, span_id: u64) -> Option<SpanData> {
        self.spans.read().get(&span_id).cloned()
    }

    pub fn current_depth(&self) -> usize {
        let thread_id = std::thread::current().id();
        let thread_stacks = self.thread_stacks.read();

        thread_stacks
            .get(&thread_id)
            .map(|stack| stack.len())
            .unwrap_or(0)
    }

    pub fn in_span(&self) -> bool {
        self.current_depth() > 0
    }

    pub fn stats(&self) -> SpanMetrics {
        let spans = self.spans.read();
        let thread_stacks = self.thread_stacks.read();

        SpanMetrics {
            total_spans: spans.len(),
            active_threads: thread_stacks.len(),
            active_spans: thread_stacks.values().map(|stack| stack.len()).sum(),
        }
    }
}

impl Default for SpanTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpanMetrics {
    pub total_spans: usize,
    pub active_threads: usize,
    pub active_spans: usize,
}

#[cfg(test)]
mod tests {
    use fd_json::json;

    use super::*;

    fn create_test_span_data(id: u64, name: &str) -> SpanData {
        SpanData {
            id,
            name: name.to_string(),
            target: "test_target".to_string(),
            level: tracing::Level::INFO,
            fields: HashMap::new(),
            file: Some("test.rs".to_string()),
            line: Some(42),
        }
    }

    #[test]
    fn test_span_lifecycle() {
        let tracker = SpanTracker::new();

        assert_eq!(tracker.current_depth(), 0);
        assert!(!tracker.in_span());
        assert!(tracker.current_context().is_none());

        let span_data = create_test_span_data(1, "test_span");
        tracker.on_new_span(span_data.clone());

        tracker.on_enter(1);
        assert_eq!(tracker.current_depth(), 1);
        assert!(tracker.in_span());

        let context = tracker.current_context().unwrap();
        assert_eq!(context.id, 1);
        assert_eq!(context.name, "test_span");
        assert_eq!(context.depth, 0);

        tracker.on_exit(1);
        assert_eq!(tracker.current_depth(), 0);
        assert!(!tracker.in_span());
        assert!(tracker.current_context().is_none());

        tracker.on_close(1);
        assert!(tracker.get_span(1).is_none());
    }

    #[test]
    fn test_nested_spans() {
        let tracker = SpanTracker::new();

        let span1 = create_test_span_data(1, "outer_span");
        let span2 = create_test_span_data(2, "inner_span");

        tracker.on_new_span(span1);
        tracker.on_new_span(span2);

        tracker.on_enter(1);
        assert_eq!(tracker.current_depth(), 1);
        let context = tracker.current_context().unwrap();
        assert_eq!(context.name, "outer_span");
        assert_eq!(context.parent_id, None);

        tracker.on_enter(2);
        assert_eq!(tracker.current_depth(), 2);
        let context = tracker.current_context().unwrap();
        assert_eq!(context.name, "inner_span");
        assert_eq!(context.parent_id, Some(1));

        let stack = tracker.current_stack();
        assert_eq!(stack.len(), 2);
        assert_eq!(stack[0].name, "outer_span");
        assert_eq!(stack[1].name, "inner_span");

        tracker.on_exit(2);
        assert_eq!(tracker.current_depth(), 1);
        let context = tracker.current_context().unwrap();
        assert_eq!(context.name, "outer_span");

        tracker.on_exit(1);
        assert_eq!(tracker.current_depth(), 0);
    }

    #[test]
    fn test_span_recording() {
        let tracker = SpanTracker::new();

        let mut span_data = create_test_span_data(1, "test_span");
        span_data
            .fields
            .insert("initial".to_string(), json!("value"));
        tracker.on_new_span(span_data);

        let mut new_fields = HashMap::new();
        new_fields.insert("recorded".to_string(), json!("data"));
        new_fields.insert("count".to_string(), json!(42));

        tracker.on_record(1, new_fields);

        let span = tracker.get_span(1).unwrap();
        assert_eq!(span.fields.get("initial"), Some(&json!("value")));
        assert_eq!(span.fields.get("recorded"), Some(&json!("data")));
        assert_eq!(span.fields.get("count"), Some(&json!(42)));
    }

    #[test]
    fn test_span_stats() {
        let tracker = SpanTracker::new();

        let initial_stats = tracker.stats();
        assert_eq!(initial_stats.total_spans, 0);
        assert_eq!(initial_stats.active_threads, 0);
        assert_eq!(initial_stats.active_spans, 0);

        let span_data = create_test_span_data(1, "test_span");
        tracker.on_new_span(span_data);
        tracker.on_enter(1);

        let stats = tracker.stats();
        assert_eq!(stats.total_spans, 1);
        assert_eq!(stats.active_threads, 1);
        assert_eq!(stats.active_spans, 1);

        let span_data2 = create_test_span_data(2, "test_span2");
        tracker.on_new_span(span_data2);
        tracker.on_enter(2);

        let stats = tracker.stats();
        assert_eq!(stats.total_spans, 2);
        assert_eq!(stats.active_threads, 1);
        assert_eq!(stats.active_spans, 2);
    }
}
