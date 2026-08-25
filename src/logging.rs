use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::sync::{LazyLock, Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum LogLevelFilter {
    #[default]
    All,
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl std::fmt::Display for LogLevelFilter {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::All => write!(f, "All Levels"),
            Self::Trace => write!(f, "TRACE & up"),
            Self::Debug => write!(f, "DEBUG & up"),
            Self::Info => write!(f, "INFO & up"),
            Self::Warn => write!(f, "WARN & up"),
            Self::Error => write!(f, "ERROR only"),
        }
    }
}

impl LogLevelFilter {
    pub const ALL: [LogLevelFilter; 6] = [
        LogLevelFilter::All,
        LogLevelFilter::Trace,
        LogLevelFilter::Debug,
        LogLevelFilter::Info,
        LogLevelFilter::Warn,
        LogLevelFilter::Error,
    ];

    pub fn matches(&self, level: &str) -> bool {
        match self {
            Self::All => true,
            Self::Trace => true,
            Self::Debug => matches!(level, "DEBUG" | "INFO" | "WARN" | "ERROR"),
            Self::Info => matches!(level, "INFO" | "WARN" | "ERROR"),
            Self::Warn => matches!(level, "WARN" | "ERROR"),
            Self::Error => matches!(level, "ERROR"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub time: String,
    pub level: String,
    pub target: String,
    pub message: String,
}

pub static LOG_BUFFER: LazyLock<Mutex<VecDeque<LogEntry>>> =
    LazyLock::new(|| Mutex::new(VecDeque::with_capacity(1000)));

pub fn push_log(entry: LogEntry) {
    if let Ok(mut buffer) = LOG_BUFFER.lock() {
        if buffer.len() >= 1000 {
            buffer.pop_front();
        }
        buffer.push_back(entry);
    }
}

pub fn recent_logs() -> Vec<LogEntry> {
    if let Ok(buffer) = LOG_BUFFER.lock() {
        buffer.iter().cloned().collect()
    } else {
        Vec::new()
    }
}

pub fn clear_logs() {
    if let Ok(mut buffer) = LOG_BUFFER.lock() {
        buffer.clear();
    }
}

#[derive(Default)]
pub struct RingBufferLayer;

struct FieldVisitor {
    message: String,
    fields: Vec<(String, String)>,
}

impl tracing::field::Visit for FieldVisitor {
    fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let mut s = format!("{value:?}");
            if s.starts_with('"') && s.ends_with('"') && s.len() >= 2 {
                s = s[1..s.len() - 1].replace("\\\"", "\"");
            }
            self.message = s;
        } else {
            self.fields
                .push((field.name().to_string(), format!("{value:?}")));
        }
    }

    fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
        if field.name() == "message" {
            self.message = value.to_string();
        } else {
            self.fields
                .push((field.name().to_string(), value.to_string()));
        }
    }
}

impl<S> tracing_subscriber::Layer<S> for RingBufferLayer
where
    S: tracing::Subscriber + for<'a> tracing_subscriber::registry::LookupSpan<'a>,
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _ctx: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut visitor = FieldVisitor {
            message: String::new(),
            fields: Vec::new(),
        };
        event.record(&mut visitor);

        let formatted_time = {
            let now = time::OffsetDateTime::now_utc();
            format!(
                "{:02}:{:02}:{:02}.{:03}",
                now.hour(),
                now.minute(),
                now.second(),
                now.millisecond()
            )
        };

        let meta = event.metadata();
        let mut full_message = visitor.message;
        if !visitor.fields.is_empty() {
            let field_strs: Vec<String> = visitor
                .fields
                .into_iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect();
            if full_message.is_empty() {
                full_message = field_strs.join(" ");
            } else {
                full_message.push(' ');
                full_message.push_str(&field_strs.join(" "));
            }
        }

        push_log(LogEntry {
            time: formatted_time,
            level: meta.level().as_str().to_string(),
            target: meta.target().to_string(),
            message: full_message,
        });
    }
}
