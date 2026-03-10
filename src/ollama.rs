use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use crate::logger;

const OLLAMA_INSTALLER_URL: &str = "https://ollama.com/download/OllamaSetup.exe";
const DEFAULT_MODEL: &str = "qwen3:4b";
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
}

pub fn detect_gpu() -> Option<GpuInfo> {
    let output = Command::new("wmic")
        .args(["path", "win32_VideoController", "get", "Name,AdapterCompatibility", "/format:csv"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.split(',').collect();
        if parts.len() >= 3 {
            let vendor = parts[1].trim().to_string();
            let name = parts[2].trim().to_string();
            if name.is_empty() {
                continue;
            }
            let v = vendor.to_lowercase();
            if v.contains("nvidia") || v.contains("amd") || v.contains("intel") {
                return Some(GpuInfo { name, vendor });
            }
        }
    }
    None
}

pub fn gpu_label() -> String {
    match detect_gpu() {
        Some(gpu) => {
            let v = gpu.vendor.to_lowercase();
            let accel = if v.contains("nvidia") {
                "CUDA"
            } else if v.contains("amd") {
                "ROCm"
            } else {
                "GPU"
            };
            format!("{} ({})", gpu.name, accel)
        }
        None => "CPU".to_string(),
    }
}

fn is_installed() -> bool {
    Command::new("ollama")
        .arg("--version")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

fn is_running() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:11434".parse().unwrap(),
        std::time::Duration::from_secs(1),
    ).is_ok()
}

pub fn ensure_ollama(app_dir: &Path) -> Result<(), String> {
    if is_installed() {
        return Ok(());
    }

    logger::log("Ollama 설치 중...");

    let installer_path = app_dir.join("OllamaSetup.exe");

    let resp = ureq::get(OLLAMA_INSTALLER_URL)
        .header("User-Agent", "dartlab-desktop")
        .call()
        .map_err(|e| format!("Ollama 다운로드 실패: {e}"))?;

    let bytes = resp.into_body()
        .with_config()
        .limit(512 * 1024 * 1024)
        .read_to_vec()
        .map_err(|e| e.to_string())?;
    std::fs::write(&installer_path, &bytes).map_err(|e| e.to_string())?;

    let status = Command::new(&installer_path)
        .arg("/VERYSILENT")
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("Ollama 설치 실행 실패: {e}"))?;

    std::fs::remove_file(&installer_path).ok();

    if !status.success() {
        return Err("Ollama 설치 실패".into());
    }

    logger::log("Ollama 설치 완료");
    Ok(())
}

pub fn ensure_serve() -> Result<(), String> {
    if is_running() {
        return Ok(());
    }

    logger::log("Ollama serve 시작 중...");
    Command::new("ollama")
        .arg("serve")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("ollama serve 실패: {e}"))?;

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(15);
    while std::time::Instant::now() < deadline {
        if is_running() {
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    Err("Ollama 서버 응답 시간 초과".into())
}

pub fn ensure_model() -> Result<(), String> {
    let output = Command::new("ollama")
        .args(["list"])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("ollama list 실패: {e}"))?;

    let stdout = String::from_utf8_lossy(&output.stdout);
    let model_base = DEFAULT_MODEL.split(':').next().unwrap_or(DEFAULT_MODEL);
    if stdout.lines().any(|l| l.contains(model_base)) {
        return Ok(());
    }

    logger::log(&format!("{DEFAULT_MODEL} 모델 다운로드 중..."));

    let status = Command::new("ollama")
        .args(["pull", DEFAULT_MODEL])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("ollama pull 실패: {e}"))?;

    if !status.success() {
        return Err(format!("{DEFAULT_MODEL} 다운로드 실패"));
    }

    logger::log("모델 다운로드 완료");
    Ok(())
}
