use crate::paths;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;

static LOG_PATH: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn init() {
    let log_dir = paths::app_dir().join("logs");
    fs::create_dir_all(&log_dir).ok();

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let path = log_dir.join(format!("dartlab-{now}.log"));
    if let Ok(mut lock) = LOG_PATH.lock() {
        *lock = Some(path);
    }

    cleanup_old_logs(&log_dir);
}

pub fn log(msg: &str) {
    let path = match LOG_PATH.lock().ok().and_then(|l| l.clone()) {
        Some(p) => p,
        None => return,
    };

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let line = format!("[{timestamp}] {msg}\n");

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

pub fn log_path() -> Option<PathBuf> {
    LOG_PATH.lock().ok().and_then(|l| l.clone())
}

fn cleanup_old_logs(log_dir: &std::path::Path) {
    let cutoff = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
        .saturating_sub(7 * 24 * 3600);

    if let Ok(entries) = fs::read_dir(log_dir) {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            let ts_str = name
                .strip_prefix("dartlab-server-")
                .and_then(|s| s.strip_suffix(".log"))
                .or_else(|| {
                    name.strip_prefix("dartlab-")
                        .and_then(|s| s.strip_suffix(".log"))
                });

            if let Some(ts_str) = ts_str {
                if let Ok(ts) = ts_str.parse::<u64>() {
                    if ts < cutoff {
                        fs::remove_file(entry.path()).ok();
                    }
                }
            }
        }
    }
}
