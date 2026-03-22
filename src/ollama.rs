use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};
use crate::logger;

const DEFAULT_MODEL: &str = "qwen3:4b";
const CREATE_NO_WINDOW: u32 = 0x08000000;
const OLLAMA_INSTALLER_URL: &str = "https://ollama.com/download/OllamaSetup.exe";

static OLLAMA_PROCESS: Mutex<Option<Child>> = Mutex::new(None);
static SPAWNED_BY_US: AtomicBool = AtomicBool::new(false);
static OLLAMA_BIN: Mutex<Option<String>> = Mutex::new(None);

pub struct GpuInfo {
    pub name: String,
    pub vendor: String,
}

pub fn detect_gpu() -> Option<GpuInfo> {
    let output = Command::new("powershell")
        .args([
            "-NoProfile", "-NoLogo", "-Command",
            "Get-CimInstance Win32_VideoController | Select-Object AdapterCompatibility,Name | ConvertTo-Csv -NoTypeInformation"
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines().skip(1) {
        let cleaned = line.replace('"', "");
        let parts: Vec<&str> = cleaned.split(',').collect();
        if parts.len() >= 2 {
            let vendor = parts[0].trim().to_string();
            let name = parts[1].trim().to_string();
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

fn resolve_ollama_bin() -> Option<String> {
    if let Ok(lock) = OLLAMA_BIN.lock() {
        if let Some(ref bin) = *lock {
            return Some(bin.clone());
        }
    }

    if Command::new("ollama")
        .arg("--version")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
    {
        if let Ok(mut lock) = OLLAMA_BIN.lock() {
            *lock = Some("ollama".to_string());
        }
        return Some("ollama".to_string());
    }

    let candidates = [
        dirs::data_local_dir().map(|d| d.join("Programs").join("Ollama").join("ollama.exe")),
        Some(std::path::PathBuf::from(r"C:\Program Files\Ollama\ollama.exe")),
        Some(std::path::PathBuf::from(r"C:\Program Files (x86)\Ollama\ollama.exe")),
    ];

    for candidate in &candidates {
        if let Some(path) = candidate {
            if path.exists() {
                let p = path.to_string_lossy().to_string();
                if let Ok(mut lock) = OLLAMA_BIN.lock() {
                    *lock = Some(p.clone());
                }
                return Some(p);
            }
        }
    }

    None
}

fn is_tcp_reachable() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:11434".parse().unwrap(),
        std::time::Duration::from_secs(1),
    ).is_ok()
}

fn is_healthy() -> bool {
    ureq::get("http://127.0.0.1:11434/api/tags")
        .call()
        .is_ok()
}

pub fn ensure_ollama(app_dir: &Path) -> Result<(), String> {
    if resolve_ollama_bin().is_some() {
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

    let output = Command::new(&installer_path)
        .arg("/VERYSILENT")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Ollama 설치 실행 실패: {e}"))?;

    std::fs::remove_file(&installer_path).ok();

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        logger::log(&format!("Ollama 설치 stderr: {stderr}"));
        return Err("Ollama 설치 실패".into());
    }

    if resolve_ollama_bin().is_none() {
        return Err("Ollama 설치 후에도 실행 파일을 찾을 수 없습니다. 시스템을 재시작해 주세요.".into());
    }

    logger::log("Ollama 설치 완료");
    Ok(())
}

pub fn ensure_serve() -> Result<(), String> {
    if is_healthy() {
        return Ok(());
    }

    if is_tcp_reachable() {
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        while std::time::Instant::now() < deadline {
            if is_healthy() {
                return Ok(());
            }
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
    }

    let bin = resolve_ollama_bin()
        .ok_or("Ollama 실행 파일을 찾을 수 없습니다")?;

    logger::log("Ollama serve 시작 중...");
    let child = Command::new(&bin)
        .arg("serve")
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()
        .map_err(|e| format!("ollama serve 실패: {e}"))?;

    if let Ok(mut lock) = OLLAMA_PROCESS.lock() {
        *lock = Some(child);
    }
    SPAWNED_BY_US.store(true, Ordering::SeqCst);

    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
    while std::time::Instant::now() < deadline {
        if is_healthy() {
            logger::log("Ollama serve 준비 완료");
            return Ok(());
        }
        std::thread::sleep(std::time::Duration::from_millis(300));
    }

    Err("Ollama 서버 응답 시간 초과 (20초)".into())
}

pub fn ensure_model() -> Result<(), String> {
    let bin = resolve_ollama_bin()
        .ok_or("Ollama 실행 파일을 찾을 수 없습니다")?;

    let output = Command::new(&bin)
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

    let output = Command::new(&bin)
        .args(["pull", DEFAULT_MODEL])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("ollama pull 실패: {e}"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        logger::log(&format!("ollama pull stderr: {stderr}"));
        return Err(format!("{DEFAULT_MODEL} 다운로드 실패"));
    }

    logger::log("모델 다운로드 완료");

    verify_model_loaded()?;

    Ok(())
}

fn verify_model_loaded() -> Result<(), String> {
    let resp = ureq::get("http://127.0.0.1:11434/api/tags")
        .call()
        .map_err(|e| format!("모델 검증 실패: {e}"))?;

    let body: String = resp.into_body()
        .read_to_string()
        .map_err(|e| e.to_string())?;

    let model_base = DEFAULT_MODEL.split(':').next().unwrap_or(DEFAULT_MODEL);
    if !body.contains(model_base) {
        return Err(format!("{DEFAULT_MODEL} 모델이 Ollama에 등록되지 않았습니다"));
    }

    Ok(())
}

pub fn stop_ollama() {
    if !SPAWNED_BY_US.load(Ordering::SeqCst) {
        return;
    }

    if let Ok(mut lock) = OLLAMA_PROCESS.lock() {
        if let Some(ref mut child) = *lock {
            child.kill().ok();
            child.wait().ok();
            logger::log("Ollama 프로세스 종료");
        }
        *lock = None;
    }
    SPAWNED_BY_US.store(false, Ordering::SeqCst);
}
