use std::path::Path;
use std::process::Command;
use crate::{paths, ui};

const UV_VERSION: &str = "0.6.14";

pub fn ensure_uv(app_dir: &Path) -> Result<(), String> {
    let uv = paths::uv_bin(app_dir);

    if uv.exists() {
        let out = Command::new(&uv).arg("--version").output();
        if out.is_ok() {
            return Ok(());
        }
    }

    ui::print_step(1, 4, "uv 다운로드 중...");

    let uv_dir = app_dir.join("uv");
    std::fs::create_dir_all(&uv_dir).map_err(|e| e.to_string())?;

    let url = format!(
        "https://github.com/astral-sh/uv/releases/download/{UV_VERSION}/uv-x86_64-pc-windows-msvc.zip"
    );

    let zip_path = uv_dir.join("uv.zip");
    download_file(&url, &zip_path)?;
    extract_zip(&zip_path, &uv_dir)?;

    let nested = uv_dir.join("uv-x86_64-pc-windows-msvc");
    if nested.exists() {
        for entry in std::fs::read_dir(&nested).map_err(|e| e.to_string())? {
            let entry = entry.map_err(|e| e.to_string())?;
            let dest = uv_dir.join(entry.file_name());
            if !dest.exists() {
                std::fs::rename(entry.path(), dest).map_err(|e| e.to_string())?;
            }
        }
        std::fs::remove_dir_all(&nested).ok();
    }

    std::fs::remove_file(&zip_path).ok();

    if !uv.exists() {
        return Err("uv.exe not found after extraction".into());
    }

    ui::print_ok("uv 설치 완료");
    Ok(())
}

pub fn ensure_dartlab(app_dir: &Path) -> Result<(), String> {
    let uv = paths::uv_bin(app_dir);
    let venv = paths::venv_dir(app_dir);

    if !venv.exists() {
        ui::print_step(2, 4, "Python 환경 생성 중...");

        let status = Command::new(&uv)
            .args(["venv", venv.to_str().unwrap(), "--python", "3.12"])
            .current_dir(app_dir)
            .status()
            .map_err(|e| e.to_string())?;

        if !status.success() {
            return Err("uv venv failed".into());
        }

        ui::print_ok("Python 환경 생성 완료");
    }

    ui::print_step(3, 4, "DartLab 설치/업데이트 중...");

    let status = Command::new(&uv)
        .args(["pip", "install", "dartlab[ai]", "--python", venv.join("Scripts").join("python.exe").to_str().unwrap()])
        .current_dir(app_dir)
        .status()
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("uv pip install dartlab[ai] failed".into());
    }

    ui::print_ok("DartLab 설치 완료");
    Ok(())
}

fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    let resp = reqwest::blocking::get(url).map_err(|e| format!("Download failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    std::fs::write(dest, &bytes).map_err(|e| e.to_string())?;
    Ok(())
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let status = Command::new("powershell")
        .args([
            "-NoProfile",
            "-Command",
            &format!(
                "Expand-Archive -Force -Path '{}' -DestinationPath '{}'",
                zip_path.display(),
                dest.display()
            ),
        ])
        .status()
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("Expand-Archive failed".into());
    }
    Ok(())
}
