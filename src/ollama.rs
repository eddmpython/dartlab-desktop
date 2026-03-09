use std::os::windows::process::CommandExt;
use std::process::Command;
use crate::ui;

const OLLAMA_DOWNLOAD_URL: &str = "https://ollama.com/download/OllamaSetup.exe";
const DEFAULT_MODEL: &str = "qwen3";
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn ensure_ollama() -> Result<(), String> {
    if is_ollama_installed() {
        ensure_model()?;
        return Ok(());
    }

    ui::print_step(4, 4, "Ollama 설치 중...");

    let temp_dir = std::env::temp_dir();
    let installer = temp_dir.join("OllamaSetup.exe");

    let resp = reqwest::blocking::get(OLLAMA_DOWNLOAD_URL)
        .map_err(|e| format!("Ollama 다운로드 실패: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    std::fs::write(&installer, &bytes).map_err(|e| e.to_string())?;

    let status = Command::new(&installer)
        .arg("/SILENT")
        .status()
        .map_err(|e| format!("Ollama 인스톨러 실행 실패: {e}"))?;

    std::fs::remove_file(&installer).ok();

    if !status.success() {
        return Err("Ollama 설치 실패".into());
    }

    std::thread::sleep(std::time::Duration::from_secs(3));

    if !is_ollama_installed() {
        return Err("Ollama 설치 후에도 감지되지 않음".into());
    }

    ui::print_ok("Ollama 설치 완료");
    ensure_model()
}

fn ensure_model() -> Result<(), String> {
    if has_model(DEFAULT_MODEL) {
        return Ok(());
    }

    ui::print_info(&format!("AI 모델 다운로드 중 ({DEFAULT_MODEL})..."));

    let status = Command::new("ollama")
        .args(["pull", DEFAULT_MODEL])
        .creation_flags(CREATE_NO_WINDOW)
        .status()
        .map_err(|e| format!("ollama pull 실패: {e}"))?;

    if !status.success() {
        return Err(format!("ollama pull {DEFAULT_MODEL} failed"));
    }

    ui::print_ok(&format!("{DEFAULT_MODEL} 모델 준비 완료"));
    Ok(())
}

fn is_ollama_installed() -> bool {
    Command::new("ollama")
        .arg("--version")
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .is_ok_and(|o| o.status.success())
}

fn has_model(model: &str) -> bool {
    let output = Command::new("ollama")
        .args(["list"])
        .creation_flags(CREATE_NO_WINDOW)
        .output();

    match output {
        Ok(o) if o.status.success() => {
            let stdout = String::from_utf8_lossy(&o.stdout);
            stdout.lines().any(|line| line.starts_with(model))
        }
        _ => false,
    }
}
