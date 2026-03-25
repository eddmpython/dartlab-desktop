use crate::{logger, paths};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::Mutex;

const PORT: u16 = 8400;
const CREATE_NO_WINDOW: u32 = 0x08000000;

static SERVER_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
static SERVER_LOG: Mutex<Option<PathBuf>> = Mutex::new(None);

pub fn is_port_in_use() -> bool {
    std::net::TcpStream::connect_timeout(
        &format!("127.0.0.1:{PORT}").parse().unwrap(),
        std::time::Duration::from_secs(1),
    )
    .is_ok()
}

fn verify_dartlab_server() -> bool {
    ureq::get(&format!("http://127.0.0.1:{PORT}"))
        .call()
        .is_ok()
}

pub fn start_server(app_dir: &Path, use_ollama: bool) -> Result<PathBuf, String> {
    if is_port_in_use() {
        if let Ok(mut lock) = SERVER_LOG.lock() {
            *lock = None;
        }
        if verify_dartlab_server() {
            let dummy = app_dir.join("logs").join("dartlab-server-existing.log");
            return Ok(dummy);
        }
        return Err(format!(
            "포트 {PORT}이 다른 프로세스에 의해 사용 중입니다. 해당 프로세스를 종료하고 다시 시도해 주세요."
        ));
    }

    let dartlab = paths::dartlab_bin(app_dir);

    let log_dir = app_dir.join("logs");
    fs::create_dir_all(&log_dir).ok();
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let server_log_path = log_dir.join(format!("dartlab-server-{ts}.log"));

    let log_file =
        fs::File::create(&server_log_path).map_err(|e| format!("서버 로그 파일 생성 실패: {e}"))?;
    let log_file_err = log_file
        .try_clone()
        .map_err(|e| format!("로그 파일 복제 실패: {e}"))?;

    let mut command = Command::new(&dartlab);
    command
        .args(["ai", "--port", &PORT.to_string(), "--no-browser"])
        .env("DARTLAB_NO_BROWSER", "1")
        .current_dir(app_dir)
        .stdout(Stdio::from(log_file))
        .stderr(Stdio::from(log_file_err))
        .creation_flags(CREATE_NO_WINDOW);

    if use_ollama {
        command.env("DARTLAB_LLM_BASE_URL", "http://127.0.0.1:11434/v1");
        command.env("DARTLAB_PRELOAD_OLLAMA", "1");
    }

    let child = command
        .spawn()
        .map_err(|e| format!("dartlab 실행 실패: {e}"))?;

    if let Ok(mut lock) = SERVER_PROCESS.lock() {
        *lock = Some(child);
    }
    if let Ok(mut lock) = SERVER_LOG.lock() {
        *lock = Some(server_log_path.clone());
    }

    logger::log(&format!(
        "dartlab 서버 시작 (로그: {})",
        server_log_path.display()
    ));
    Ok(server_log_path)
}

fn read_tail(path: &Path, lines: usize) -> String {
    let content = match fs::read_to_string(path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };
    let all: Vec<&str> = content.lines().collect();
    let start = all.len().saturating_sub(lines);
    all[start..].join("\n")
}

pub fn server_log_path() -> Option<PathBuf> {
    SERVER_LOG.lock().ok().and_then(|l| l.clone())
}

pub fn wait_for_server(timeout_secs: u64) -> Result<(), String> {
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(timeout_secs);
    let log_path = server_log_path();

    while std::time::Instant::now() < deadline {
        if let Ok(mut lock) = SERVER_PROCESS.lock() {
            if let Some(ref mut child) = *lock {
                if let Some(exit) = child.try_wait().ok().flatten() {
                    let mut msg = format!("서버 프로세스 종료됨 (code: {exit})");
                    if let Some(ref lp) = log_path {
                        let tail = read_tail(lp, 15);
                        if !tail.is_empty() {
                            msg.push_str(&format!("\n\n--- 서버 로그 ---\n{tail}"));
                        }
                        msg.push_str(&format!("\n\n로그 파일: {}", lp.display()));
                    }
                    return Err(msg);
                }
            }
        }

        if std::net::TcpStream::connect_timeout(
            &format!("127.0.0.1:{PORT}").parse().unwrap(),
            std::time::Duration::from_secs(1),
        )
        .is_ok()
        {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    let mut msg = "서버 응답 시간 초과".to_string();
    if let Some(ref lp) = log_path {
        let tail = read_tail(lp, 15);
        if !tail.is_empty() {
            msg.push_str(&format!("\n\n--- 서버 로그 ---\n{tail}"));
        }
        msg.push_str(&format!("\n\n로그 파일: {}", lp.display()));
    }
    Err(msg)
}

pub fn stop_server() {
    if let Ok(mut lock) = SERVER_PROCESS.lock() {
        if let Some(ref mut child) = *lock {
            child.kill().ok();
            child.wait().ok();
        }
        *lock = None;
    }
    if let Ok(mut lock) = SERVER_LOG.lock() {
        *lock = None;
    }
}
