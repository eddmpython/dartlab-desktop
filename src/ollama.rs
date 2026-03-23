use crate::{logger, net};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

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
            if is_valid_cached_bin(bin) {
                return Some(bin.clone());
            }
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
        Some(std::path::PathBuf::from(
            r"C:\Program Files\Ollama\ollama.exe",
        )),
        Some(std::path::PathBuf::from(
            r"C:\Program Files (x86)\Ollama\ollama.exe",
        )),
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

fn is_valid_cached_bin(bin: &str) -> bool {
    if bin == "ollama" {
        return Command::new("ollama")
            .arg("--version")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
    }

    std::path::Path::new(bin).exists()
}

fn is_tcp_reachable() -> bool {
    std::net::TcpStream::connect_timeout(
        &"127.0.0.1:11434".parse().unwrap(),
        std::time::Duration::from_secs(1),
    )
    .is_ok()
}

fn is_healthy() -> bool {
    ureq::get("http://127.0.0.1:11434/api/tags").call().is_ok()
}

fn fetch_model_names() -> Result<Vec<String>, String> {
    let resp = ureq::get("http://127.0.0.1:11434/api/tags")
        .call()
        .map_err(|e| format!("Ollama tags 조회 실패: {e}"))?;

    let body = resp
        .into_body()
        .read_to_string()
        .map_err(|e| e.to_string())?;

    let json: serde_json::Value =
        serde_json::from_str(&body).map_err(|e| format!("Ollama tags 파싱 실패: {e}"))?;

    let models = json
        .get("models")
        .and_then(|v| v.as_array())
        .ok_or("Ollama tags 응답에 models 필드가 없습니다".to_string())?;

    Ok(models
        .iter()
        .filter_map(|model| model.get("name").and_then(|v| v.as_str()))
        .map(str::to_string)
        .collect())
}

fn has_model(model_name: &str) -> Result<bool, String> {
    Ok(fetch_model_names()?.iter().any(|name| name == model_name))
}

pub fn is_installed() -> bool {
    resolve_ollama_bin().is_some()
}

pub fn ensure_ollama(app_dir: &Path) -> Result<(), String> {
    if resolve_ollama_bin().is_some() {
        return Ok(());
    }

    logger::log("Ollama 설치 중...");

    let installer_path = app_dir.join("OllamaSetup.exe");

    net::download_to_file(
        OLLAMA_INSTALLER_URL,
        &installer_path,
        &[("User-Agent", "dartlab-desktop")],
        Some(512 * 1024 * 1024),
    )
    .map_err(|e| format!("Ollama 다운로드 실패: {e}"))?;

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
        return Err(
            "Ollama 설치 후에도 실행 파일을 찾을 수 없습니다. 시스템을 재시작해 주세요.".into(),
        );
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

    let bin = resolve_ollama_bin().ok_or("Ollama 실행 파일을 찾을 수 없습니다")?;

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
    let bin = resolve_ollama_bin().ok_or("Ollama 실행 파일을 찾을 수 없습니다")?;

    if has_model(DEFAULT_MODEL)? {
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
    if !has_model(DEFAULT_MODEL)? {
        return Err(format!(
            "{DEFAULT_MODEL} 모델이 Ollama에 등록되지 않았습니다"
        ));
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

pub fn uninstall_ollama() -> Result<(), String> {
    stop_ollama();

    logger::log("Ollama 제거 시작");

    let cleanup_startup = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NoLogo",
            "-Command",
            "$startup = [Environment]::GetFolderPath('Startup'); $link = Join-Path $startup 'Ollama.lnk'; if (Test-Path $link) { Remove-Item $link -Force }",
        ])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    if let Ok(output) = cleanup_startup {
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            logger::log(&format!("Ollama 시작프로그램 정리 stderr: {stderr}"));
        }
    }

    let script = r#"
      $paths = @(
        'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\Software\Microsoft\Windows\CurrentVersion\Uninstall\*',
        'HKLM:\Software\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\*'
      )
      $entry = Get-ItemProperty $paths -ErrorAction SilentlyContinue |
        Where-Object { $_.DisplayName -eq 'Ollama' } |
        Select-Object -First 1
      if (-not $entry) { exit 2 }
      $cmd = if ($entry.QuietUninstallString) { $entry.QuietUninstallString } else { $entry.UninstallString }
      if (-not $cmd) { exit 3 }
      & cmd.exe /c $cmd
      exit $LASTEXITCODE
    "#;

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NoLogo", "-Command", script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("Ollama 제거 실행 실패: {e}"))?;

    if !output.status.success() {
        let code = output.status.code().unwrap_or(-1);
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        logger::log(&format!("Ollama 제거 stdout: {stdout}"));
        logger::log(&format!("Ollama 제거 stderr: {stderr}"));
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(format!("Ollama 제거 실패 (code: {code}) {detail}"));
    }

    if let Ok(mut lock) = OLLAMA_BIN.lock() {
        *lock = None;
    }

    if is_installed() {
        return Err("Ollama 제거 후에도 실행 파일이 남아 있습니다".into());
    }

    logger::log("Ollama 제거 완료");
    Ok(())
}
