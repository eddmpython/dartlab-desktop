use crate::paths;
use std::path::Path;

const STATE_FILE: &str = "state.json";
const WARM_THRESHOLD_SECS: u64 = 7 * 24 * 3600;

pub fn quick_health_check(app_dir: &Path) -> bool {
    paths::dartlab_bin(app_dir).exists()
        && paths::python_bin(app_dir).exists()
        && paths::uv_bin(app_dir).exists()
}

pub fn clear_state(app_dir: &Path) {
    let mut json = load_state(app_dir);
    if let Some(obj) = json.as_object_mut() {
        obj.remove("last_success");
        if obj.is_empty() {
            std::fs::remove_file(app_dir.join(STATE_FILE)).ok();
            return;
        }
    }
    save_state(app_dir, &json);
}

pub fn ollama_enabled(app_dir: &Path) -> bool {
    load_state(app_dir)
        .get("ollama_enabled")
        .and_then(|v| v.as_bool())
        .unwrap_or(true)
}

pub fn set_ollama_enabled(app_dir: &Path, enabled: bool) {
    let mut json = load_state(app_dir);
    json["ollama_enabled"] = serde_json::Value::Bool(enabled);
    save_state(app_dir, &json);
}

pub fn is_warm(app_dir: &Path) -> bool {
    let json = load_state(app_dir);

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

    let mut json = load_state(app_dir);
    json["last_success"] = serde_json::Value::Number(now.into());
    save_state(app_dir, &json);
}

fn load_state(app_dir: &Path) -> serde_json::Value {
    let path = app_dir.join(STATE_FILE);
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return serde_json::json!({}),
    };

    serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({}))
}

fn save_state(app_dir: &Path, json: &serde_json::Value) {
    if let Ok(content) = serde_json::to_string(json) {
        let _ = std::fs::write(app_dir.join(STATE_FILE), content);
    }
}
