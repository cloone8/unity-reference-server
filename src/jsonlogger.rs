use log::{Level, LevelFilter, Log, SetLoggerError};
use serde::Serialize;
use std::io::Write;
use std::sync::Mutex;

#[derive(Debug)]
pub struct JsonLogger<W> {
    level: LevelFilter,
    //TODO: This doesn't play nice with async. Find some way to be non-blocking
    writer: Mutex<W>,
}

#[derive(Debug, Clone, Serialize)]
struct JsonLog {
    pub level: SerializedLogLevel,
    pub timestamp: String,
    pub message: String,
    pub file: Option<String>,
    pub line: Option<u32>,
}

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "lowercase")]
enum SerializedLogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl From<Level> for SerializedLogLevel {
    fn from(value: Level) -> Self {
        match value {
            Level::Error => Self::Error,
            Level::Warn => Self::Warn,
            Level::Info => Self::Info,
            Level::Debug => Self::Debug,
            Level::Trace => Self::Trace,
        }
    }
}

impl<W: Write + Send + 'static> JsonLogger<W> {
    pub fn init(level: LevelFilter, writer: W) -> Result<(), SetLoggerError> {
        let logger = Self {
            level,
            writer: Mutex::new(writer),
        };

        log::set_max_level(level);
        log::set_boxed_logger(Box::new(logger))?;

        Ok(())
    }
}

impl<W: Write + Send> Log for JsonLogger<W> {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        metadata.level() <= self.level
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let body = format!("{}", record.args());

        let log = JsonLog {
            level: record.level().into(),
            timestamp: chrono::Local::now().to_rfc3339(),
            file: record.file().map(str::to_owned),
            line: record.line(),
            message: body,
        };

        let mut writer = self.writer.lock().unwrap();
        _ = serde_json::to_writer(&mut *writer, &log);
        _ = writer.write(b"\n");
        _ = writer.flush();
    }

    fn flush(&self) {
        _ = self.writer.lock().unwrap().flush();
    }
}
