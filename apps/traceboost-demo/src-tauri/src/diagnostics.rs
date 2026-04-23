use std::{
    collections::VecDeque,
    fs,
    io::{self, Write},
    path::{Path, PathBuf},
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use chrono::{DateTime, Local, Utc};
use log::{Level, debug, error, info, warn};
use serde::Serialize;
use serde_json::{Map, Value, json};
use tauri::{AppHandle, Emitter};
use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

pub const DIAGNOSTICS_EVENT_NAME: &str = "diagnostics:event";
const MAX_RECENT_EVENTS: usize = 512;
const MAX_RETAINED_SESSION_FILES: usize = 20;
const MAX_RETAINED_SESSION_BYTES: u64 = 100 * 1024 * 1024;
const MAX_MESSAGE_LEN: usize = 512;
const MAX_FIELD_VALUE_LEN: usize = 1024;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsEvent {
    pub session_id: String,
    pub operation_id: String,
    pub command: String,
    pub stage: String,
    pub level: String,
    pub timestamp: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub duration_ms: Option<u128>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Map<String, Value>>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticsStatus {
    pub session_id: String,
    pub session_started_at: String,
    pub verbose_enabled: bool,
    pub session_log_path: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportBundleResponse {
    pub bundle_path: String,
}

#[derive(Debug, Clone)]
pub struct OperationToken {
    command: &'static str,
    operation_id: String,
    started_at: Instant,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AppBundleMetadata {
    app_name: String,
    app_version: String,
    app_identifier: String,
    tauri_version: &'static str,
    session_id: String,
    session_started_at: String,
    session_log_path: String,
    verbose_enabled: bool,
    platform: String,
    arch: String,
}

pub struct DiagnosticsState {
    session_id: String,
    session_started_at: String,
    verbose_enabled: AtomicBool,
    operation_counter: AtomicU64,
    session_log_path: PathBuf,
    recent_events: Mutex<VecDeque<DiagnosticsEvent>>,
}

impl DiagnosticsState {
    pub fn session_basename() -> String {
        format!(
            "traceboost-session-{}-{}",
            unix_timestamp_millis(),
            std::process::id()
        )
    }

    pub fn initialize(log_dir: &Path, session_basename: String) -> Result<Self, String> {
        fs::create_dir_all(&log_dir).map_err(|error| error.to_string())?;

        prune_session_logs(log_dir).map_err(|error| error.to_string())?;

        let session_log_path = resolve_session_log_path(log_dir, &session_basename);

        Ok(Self {
            session_id: format!("session-{}", unix_timestamp_millis()),
            session_started_at: now_rfc3339(),
            verbose_enabled: AtomicBool::new(false),
            operation_counter: AtomicU64::new(0),
            session_log_path,
            recent_events: Mutex::new(VecDeque::with_capacity(MAX_RECENT_EVENTS)),
        })
    }

    pub fn status(&self) -> DiagnosticsStatus {
        DiagnosticsStatus {
            session_id: self.session_id.clone(),
            session_started_at: self.session_started_at.clone(),
            verbose_enabled: self.verbose_enabled.load(Ordering::Relaxed),
            session_log_path: self.session_log_path.display().to_string(),
        }
    }

    pub fn set_verbose_enabled(&self, enabled: bool) {
        self.verbose_enabled.store(enabled, Ordering::Relaxed);
    }

    pub fn verbose_enabled(&self) -> bool {
        self.verbose_enabled.load(Ordering::Relaxed)
    }

    pub fn session_log_path(&self) -> &Path {
        &self.session_log_path
    }

    pub fn start_operation(
        &self,
        app: &AppHandle,
        command: &'static str,
        message: impl Into<String>,
        fields: Option<Map<String, Value>>,
    ) -> OperationToken {
        let operation_number = self.operation_counter.fetch_add(1, Ordering::Relaxed) + 1;
        let token = OperationToken {
            command,
            operation_id: format!("{}-{:04}", self.session_id, operation_number),
            started_at: Instant::now(),
        };

        self.emit_event(
            app,
            &token,
            "started",
            Level::Info,
            message.into(),
            None,
            fields,
        );
        token
    }

    pub fn progress(
        &self,
        app: &AppHandle,
        token: &OperationToken,
        message: impl Into<String>,
        fields: Option<Map<String, Value>>,
    ) {
        self.emit_event(
            app,
            token,
            "progress",
            Level::Info,
            message.into(),
            None,
            fields,
        );
    }

    pub fn verbose_progress(
        &self,
        app: &AppHandle,
        token: &OperationToken,
        message: impl Into<String>,
        fields: Option<Map<String, Value>>,
    ) {
        if self.verbose_enabled() {
            self.emit_event(
                app,
                token,
                "progress",
                Level::Debug,
                message.into(),
                None,
                fields,
            );
        }
    }

    pub fn complete(
        &self,
        app: &AppHandle,
        token: &OperationToken,
        message: impl Into<String>,
        fields: Option<Map<String, Value>>,
    ) {
        self.emit_event(
            app,
            token,
            "completed",
            Level::Info,
            message.into(),
            Some(token.started_at.elapsed().as_millis()),
            fields,
        );
    }

    pub fn fail(
        &self,
        app: &AppHandle,
        token: &OperationToken,
        message: impl Into<String>,
        fields: Option<Map<String, Value>>,
    ) {
        self.emit_event(
            app,
            token,
            "failed",
            Level::Error,
            message.into(),
            Some(token.started_at.elapsed().as_millis()),
            fields,
        );
    }

    pub fn emit_session_event(
        &self,
        app: &AppHandle,
        stage: &'static str,
        level: Level,
        message: impl Into<String>,
        fields: Option<Map<String, Value>>,
    ) {
        let token = OperationToken {
            command: "session",
            operation_id: self.session_id.clone(),
            started_at: Instant::now(),
        };
        self.emit_event(app, &token, stage, level, message.into(), None, fields);
    }

    pub fn export_bundle(
        &self,
        app: &AppHandle,
        include_sensitive_paths: bool,
    ) -> Result<PathBuf, String> {
        let bundle_path = self
            .session_log_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(format!("traceboost-support-{}.zip", self.session_id));

        let session_events = self
            .recent_events
            .lock()
            .map_err(|_| "failed to lock diagnostics events")?;
        let metadata = AppBundleMetadata {
            app_name: app.package_info().name.clone(),
            app_version: app.package_info().version.to_string(),
            app_identifier: app.config().identifier.clone(),
            tauri_version: tauri::VERSION,
            session_id: self.session_id.clone(),
            session_started_at: self.session_started_at.clone(),
            session_log_path: maybe_redact_path(
                &self.session_log_path.display().to_string(),
                include_sensitive_paths,
            ),
            verbose_enabled: self.verbose_enabled(),
            platform: std::env::consts::OS.to_string(),
            arch: std::env::consts::ARCH.to_string(),
        };

        let file = fs::File::create(&bundle_path).map_err(|error| error.to_string())?;
        let mut zip = ZipWriter::new(file);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);

        if self.session_log_path.exists() {
            zip.start_file("session.log", options)
                .map_err(|error| error.to_string())?;
            let bytes = fs::read(&self.session_log_path).map_err(|error| error.to_string())?;
            zip.write_all(&bytes).map_err(|error| error.to_string())?;
        }

        zip.start_file("session-events.json", options)
            .map_err(|error| error.to_string())?;
        zip.write_all(
            serde_json::to_vec_pretty(&*session_events)
                .map_err(|error| error.to_string())?
                .as_slice(),
        )
        .map_err(|error| error.to_string())?;

        zip.start_file("app.json", options)
            .map_err(|error| error.to_string())?;
        zip.write_all(
            serde_json::to_vec_pretty(&metadata)
                .map_err(|error| error.to_string())?
                .as_slice(),
        )
        .map_err(|error| error.to_string())?;

        zip.finish().map_err(|error| error.to_string())?;
        Ok(bundle_path)
    }

    fn emit_event(
        &self,
        app: &AppHandle,
        token: &OperationToken,
        stage: &'static str,
        level: Level,
        message: String,
        duration_ms: Option<u128>,
        fields: Option<Map<String, Value>>,
    ) {
        let event = DiagnosticsEvent {
            session_id: self.session_id.clone(),
            operation_id: token.operation_id.clone(),
            command: token.command.to_string(),
            stage: stage.to_string(),
            level: level.as_str().to_ascii_lowercase(),
            timestamp: now_rfc3339(),
            message: sanitize_message(&message),
            duration_ms,
            fields: sanitize_fields(fields),
        };

        if let Ok(mut recent_events) = self.recent_events.lock() {
            if recent_events.len() >= MAX_RECENT_EVENTS {
                recent_events.pop_front();
            }
            recent_events.push_back(event.clone());
        }

        let log_line = format_log_line(&event);
        match level {
            Level::Error => error!("{log_line}"),
            Level::Warn => warn!("{log_line}"),
            Level::Info => info!("{log_line}"),
            Level::Debug => debug!("{log_line}"),
            Level::Trace => debug!("{log_line}"),
        }

        if let Err(error) = app.emit(DIAGNOSTICS_EVENT_NAME, event) {
            error!("failed to emit diagnostics event: {error}");
        }
    }
}

pub fn build_fields(
    entries: impl IntoIterator<Item = (&'static str, Value)>,
) -> Map<String, Value> {
    entries
        .into_iter()
        .map(|(key, value)| (key.to_string(), value))
        .collect()
}

fn now_rfc3339() -> String {
    let utc: DateTime<Utc> = SystemTime::now().into();
    utc.with_timezone(&Local).to_rfc3339()
}

fn unix_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
}

fn format_log_line(event: &DiagnosticsEvent) -> String {
    let fields = event
        .fields
        .as_ref()
        .and_then(|fields| serde_json::to_string(fields).ok())
        .unwrap_or_else(|| "{}".to_string());

    format!(
        "session={} op={} command={} stage={} duration_ms={} message={} fields={}",
        event.session_id,
        event.operation_id,
        event.command,
        event.stage,
        event.duration_ms.unwrap_or_default(),
        event.message,
        fields
    )
}

fn sanitize_message(message: &str) -> String {
    let single_line = message.replace(['\r', '\n'], " ");
    truncate_string(single_line.trim(), MAX_MESSAGE_LEN)
}

fn sanitize_fields(fields: Option<Map<String, Value>>) -> Option<Map<String, Value>> {
    fields.map(|fields| {
        fields
            .into_iter()
            .map(|(key, value)| {
                let redact = key.to_ascii_lowercase().contains("path");
                (key, sanitize_value(value, redact))
            })
            .collect()
    })
}

fn sanitize_value(value: Value, redact: bool) -> Value {
    match value {
        Value::String(text) => {
            let text = text.replace(['\r', '\n'], " ");
            if redact {
                Value::String("[redacted-path]".to_string())
            } else {
                Value::String(truncate_string(text.trim(), MAX_FIELD_VALUE_LEN))
            }
        }
        Value::Array(values) => Value::Array(
            values
                .into_iter()
                .map(|item| sanitize_value(item, redact))
                .collect(),
        ),
        Value::Object(map) => Value::Object(
            map.into_iter()
                .map(|(key, value)| {
                    let child_redact = redact || key.to_ascii_lowercase().contains("path");
                    (key, sanitize_value(value, child_redact))
                })
                .collect(),
        ),
        other => other,
    }
}

fn maybe_redact_path(path: &str, include_sensitive_paths: bool) -> String {
    if include_sensitive_paths {
        path.to_string()
    } else {
        "[redacted-path]".to_string()
    }
}

fn truncate_string(value: &str, max_len: usize) -> String {
    if value.chars().count() <= max_len {
        return value.to_string();
    }
    value.chars().take(max_len).collect()
}

fn resolve_session_log_path(log_dir: &Path, session_basename: &str) -> PathBuf {
    let matching_path = fs::read_dir(log_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.flatten())
        .map(|entry| entry.path())
        .find(|path| {
            path.file_stem()
                .and_then(|value| value.to_str())
                .is_some_and(|name| name.starts_with(session_basename))
        });

    matching_path.unwrap_or_else(|| log_dir.join(format!("{session_basename}.log")))
}

fn prune_session_logs(log_dir: &Path) -> io::Result<()> {
    let mut session_logs = fs::read_dir(log_dir)?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            let file_name = path.file_name()?.to_str()?;
            if !file_name.starts_with("traceboost-session-") {
                return None;
            }

            let metadata = entry.metadata().ok()?;
            Some((path, metadata.modified().ok()?, metadata.len()))
        })
        .collect::<Vec<_>>();

    session_logs.sort_by(|left, right| right.1.cmp(&left.1));

    let mut total_bytes = 0_u64;
    for (index, (path, _, len)) in session_logs.into_iter().enumerate() {
        total_bytes += len;
        if index >= MAX_RETAINED_SESSION_FILES || total_bytes > MAX_RETAINED_SESSION_BYTES {
            let _ = fs::remove_file(path);
        }
    }

    Ok(())
}

pub fn json_value<T>(value: T) -> Value
where
    T: Serialize,
{
    json!(value)
}
