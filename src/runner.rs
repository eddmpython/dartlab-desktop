use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use crate::paths;

const PORT: u16 = 8400;
const CREATE_NO_WINDOW: u32 = 0x08000000;

static SERVER_PROCESS: Mutex<Option<Child>> = Mutex::new(None);

pub fn start_server(app_dir: &Path) -> Result<(), String> {
    let dartlab = paths::dartlab_bin(app_dir);

    let child = Command::new(&dartlab)
        .args(["ai", "--port", &PORT.to_string(), "--no-browser"])
        .current_dir(app_dir)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Ok(mut lock) = SERVER_PROCESS.lock() {
        *lock = Some(child);
    }

    Ok(())
}

pub fn wait_for_server(timeout_secs: u64) -> bool {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();

    let url = format!("http://localhost:{PORT}/api/status");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    while std::time::Instant::now() < deadline {
        if client.get(&url).send().is_ok_and(|r| r.status().is_success()) {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    false
}

pub fn stop_server() {
    if let Ok(mut lock) = SERVER_PROCESS.lock() {
        if let Some(ref mut child) = *lock {
            child.kill().ok();
            child.wait().ok();
        }
        *lock = None;
    }
}
