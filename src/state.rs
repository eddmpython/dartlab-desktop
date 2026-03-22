use std::path::Path;
use crate::paths;

const STATE_FILE: &str = "state.json";
const WARM_THRESHOLD_SECS: u64 = 7 * 24 * 3600;

pub fn quick_health_check(app_dir: &Path) -> bool {
    paths::dartlab_bin(app_dir).exists()
        && paths::python_bin(app_dir).exists()
        && paths::uv_bin(app_dir).exists()
}

pub fn clear_state(app_dir: &Path) {
    let path = app_dir.join(STATE_FILE);
    std::fs::remove_file(path).ok();
}

pub fn is_warm(app_dir: &Path) -> bool {
    let path = app_dir.join(STATE_FILE);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return false,
    };

    let json: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return false,
    };

    let ts = match json["last_success"].as_u64() {
        Some(t) => t,
        None => return false,
    };

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    now.saturating_sub(ts) < WARM_THRESHOLD_SECS
}

pub fn mark_success(app_dir: &Path) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    let json = format!("{{\"last_success\":{now}}}");
    let _ = std::fs::write(app_dir.join(STATE_FILE), json);
}
