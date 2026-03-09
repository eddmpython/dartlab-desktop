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

    let log_file = app_dir.join("server.log");
    let stderr_file = std::fs::File::create(&log_file).map_err(|e| e.to_string())?;
    let stdout_file = stderr_file.try_clone().map_err(|e| e.to_string())?;

    let child = Command::new(&dartlab)
        .args(["ai", "--port", &PORT.to_string(), "--no-browser"])
        .env("DARTLAB_NO_BROWSER", "1")
        .current_dir(app_dir)
        .stdout(Stdio::from(stdout_file))
        .stderr(Stdio::from(stderr_file))
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| e.to_string())?;

    if let Ok(mut lock) = SERVER_PROCESS.lock() {
        *lock = Some(child);
    }

    Ok(())
}

pub fn wait_for_server(timeout_secs: u64) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();

    let url = format!("http://localhost:{PORT}/api/status");
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    while std::time::Instant::now() < deadline {
        if let Ok(mut lock) = SERVER_PROCESS.lock() {
            if let Some(ref mut child) = *lock {
                if let Some(exit) = child.try_wait().ok().flatten() {
                    let app_dir = paths::app_dir();
                    let log_path = app_dir.join("server.log");
                    let log_tail = std::fs::read_to_string(&log_path)
                        .unwrap_or_default()
                        .lines()
                        .rev()
                        .take(5)
                        .collect::<Vec<_>>()
                        .into_iter()
                        .rev()
                        .collect::<Vec<_>>()
                        .join("\n");
                    return Err(format!("서버 프로세스 종료됨 (code: {exit})\n{log_tail}"));
                }
            }
        }

        if client.get(&url).send().is_ok_and(|r| r.status().is_success()) {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(500));
    }

    let app_dir = paths::app_dir();
    let log_path = app_dir.join("server.log");
    let log_tail = std::fs::read_to_string(&log_path)
        .unwrap_or_default()
        .lines()
        .rev()
        .take(5)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");
    Err(format!("서버 응답 시간 초과 ({timeout_secs}초)\n{log_tail}"))
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
