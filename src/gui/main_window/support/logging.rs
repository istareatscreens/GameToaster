use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug)]
pub struct LogOpenError {
    pub path: PathBuf,
    pub error: std::io::Error,
}

impl std::fmt::Display for LogOpenError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "failed to open launcher log {}: {}",
            self.path.display(),
            self.error
        )
    }
}

#[derive(Clone)]
pub struct LauncherLog {
    file: Arc<Mutex<File>>,

    enabled: Arc<AtomicBool>,
}

impl LauncherLog {
    pub fn open(launcher_dir: &Path) -> Result<Self, LogOpenError> {
        let path = launcher_dir.join("GameToaster.log");

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
            .map_err(|error| LogOpenError { path, error })?;
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
            enabled: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn event(&self, operation: &str, fields: &[(&str, String)]) {
        if !self.enabled.load(Ordering::Relaxed) {
            return;
        }
        self.write_line(operation, fields);
    }

    pub fn crash(&self, operation: &str, fields: &[(&str, String)]) {
        self.write_line(operation, fields);
    }

    fn write_line(&self, operation: &str, fields: &[(&str, String)]) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let mut line = format!("ts={timestamp} operation={}", token(operation));
        for (key, value) in fields {
            line.push(' ');
            line.push_str(&token(key));
            line.push('=');
            line.push_str(&quoted(value));
        }
        line.push('\n');
        if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(line.as_bytes());
            let _ = file.flush();
        }
    }
}

pub struct BackendLogSink {
    log: LauncherLog,
}

const BACKEND_LOG_TARGETS: &[&str] = &[
    "eframe",
    "egui_winit",
    "egui_glow",
    "winit",
    "glutin",
    "glutin_winit",
];

impl BackendLogSink {
    pub fn new(log: LauncherLog) -> Self {
        Self { log }
    }

    pub fn install(log: &LauncherLog) {
        if log::set_boxed_logger(Box::new(Self::new(log.clone()))).is_ok() {
            log::set_max_level(log::LevelFilter::Warn);
        }
    }

    pub fn accepts(target: &str, level: log::Level) -> bool {
        level <= log::Level::Warn
            && BACKEND_LOG_TARGETS
                .iter()
                .any(|prefix| target == *prefix || target.starts_with(&format!("{prefix}::")))
    }
}

impl log::Log for BackendLogSink {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        Self::accepts(metadata.target(), metadata.level())
    }

    fn log(&self, record: &log::Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        self.log.crash(
            "backend-log",
            &[
                ("level", record.level().to_string()),
                ("target", record.target().to_string()),
                ("message", record.args().to_string()),
            ],
        );
    }

    fn flush(&self) {}
}

fn token(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | '.') {
                character
            } else {
                '_'
            }
        })
        .collect()
}

fn quoted(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

#[cfg(test)]
#[path = "../../../../tests/unit/gui/main_window/support/logging__tests.rs"]
mod tests;
