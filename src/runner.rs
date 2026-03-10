use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;
use crate::paths;

const PORT: u16 = 8400;
const CREATE_NO_WINDOW: u32 = 0x08000000;

static SERVER_PROCESS: Mutex<Option<Child>> = Mutex::new(None);

pub fn is_port_in_use() -> bool {
    std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{PORT}").parse().unwrap(),
        std::time::Duration::from_secs(1),
    ).is_ok()
}

pub fn start_server(app_dir: &Path) -> Result<(), String> {
    if is_port_in_use() {
        return Ok(());
    }

    let dartlab = paths::dartlab_bin(app_dir);

    let child = Command::new(&dartlab)
        .args(["ai", "--port", &PORT.to_string(), "--no-browser"])
        .env("DARTLAB_NO_BROWSER", "1")
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

pub fn wait_for_server(timeout_secs: u64) -> Result<(), String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);

    while std::time::Instant::now() < deadline {
        if let Ok(mut lock) = SERVER_PROCESS.lock() {
            if let Some(ref mut child) = *lock {
                if let Some(exit) = child.try_wait().ok().flatten() {
                    return Err(format!("서버 프로세스 종료됨 (code: {exit})"));
                }
            }
        }

        if std::net::TcpStream::connect_timeout(
            &format!("127.0.0.1:{PORT}").parse().unwrap(),
            std::time::Duration::from_secs(1),
        ).is_ok() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    Err("서버 응답 시간 초과".into())
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
