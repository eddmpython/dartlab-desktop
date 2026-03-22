use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;
use crate::{logger, paths};

const UV_VERSION: &str = "0.6.14";
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn ensure_uv(app_dir: &Path) -> Result<(), String> {
    let uv = paths::uv_bin(app_dir);

    if uv.exists() {
        let ok = Command::new(&uv)
            .arg("--version")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if ok {
            return Ok(());
        }
        logger::log("uv.exe 존재하지만 실행 실패 — 재설치");
        let uv_dir = app_dir.join("uv");
        std::fs::remove_dir_all(&uv_dir).ok();
    }

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

    Ok(())
}

pub fn ensure_dartlab(app_dir: &Path) -> Result<(), String> {
    let uv = paths::uv_bin(app_dir);
    let venv = paths::venv_dir(app_dir);
    let dartlab_bin = paths::dartlab_bin(app_dir);

    cleanup_legacy(app_dir);

    let needs_venv = if venv.exists() {
        !venv.join("Scripts").join("python.exe").exists()
    } else {
        true
    };

    if needs_venv {
        if venv.exists() {
            std::fs::remove_dir_all(&venv).ok();
        }

        let output = Command::new(&uv)
            .args(["venv", venv.to_str().unwrap(), "--python", "3.12"])
            .current_dir(app_dir)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            logger::log(&format!("uv venv stderr: {stderr}"));
            return Err(format!("uv venv failed: {stderr}"));
        }
    }

    let needs_install = if dartlab_bin.exists() {
        let ok = Command::new(&dartlab_bin)
            .arg("--version")
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);
        if !ok {
            logger::log("dartlab.exe 존재하지만 실행 실패 — 재설치");
            std::fs::remove_dir_all(&venv).ok();
            let output = Command::new(&uv)
                .args(["venv", venv.to_str().unwrap(), "--python", "3.12"])
                .current_dir(app_dir)
                .creation_flags(CREATE_NO_WINDOW)
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                let stderr = String::from_utf8_lossy(&output.stderr);
                logger::log(&format!("uv venv stderr: {stderr}"));
                return Err(format!("uv venv failed: {stderr}"));
            }
            true
        } else {
            false
        }
    } else {
        true
    };

    if needs_install {
        let python = paths::python_bin(app_dir);
        let output = Command::new(&uv)
            .args(["pip", "install", "dartlab[ai,llm]", "--python", python.to_str().unwrap()])
            .current_dir(app_dir)
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .map_err(|e| e.to_string())?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            logger::log(&format!("uv pip install stderr: {stderr}"));
            return Err(format!("uv pip install dartlab[ai,llm] failed: {stderr}"));
        }
    }

    Ok(())
}

fn cleanup_legacy(app_dir: &Path) {
    let pyproject = app_dir.join("pyproject.toml");
    if pyproject.exists() {
        std::fs::remove_file(&pyproject).ok();
    }
    let uv_lock = app_dir.join("uv.lock");
    if uv_lock.exists() {
        std::fs::remove_file(&uv_lock).ok();
    }
    let old_venv = app_dir.join("venv");
    if old_venv.exists() {
        std::fs::remove_dir_all(&old_venv).ok();
    }
}

fn download_file(url: &str, dest: &Path) -> Result<(), String> {
    let mut last_err = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            logger::log(&format!("다운로드 재시도 ({}/3): {url}", attempt + 1));
            std::thread::sleep(std::time::Duration::from_secs(3));
        }

        match ureq::get(url).call() {
            Ok(resp) => {
                let status = resp.status();
                if status.as_u16() < 200 || status.as_u16() >= 300 {
                    last_err = format!("HTTP {}", status);
                    continue;
                }
                let bytes = resp.into_body().read_to_vec().map_err(|e| e.to_string())?;
                std::fs::write(dest, &bytes).map_err(|e| e.to_string())?;
                return Ok(());
            }
            Err(e) => {
                last_err = format!("Download failed: {e}");
            }
        }
    }
    Err(last_err)
}

fn extract_zip(zip_path: &Path, dest: &Path) -> Result<(), String> {
    let file = std::fs::File::open(zip_path).map_err(|e| e.to_string())?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| e.to_string())?;

    for i in 0..archive.len() {
        let mut entry = archive.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();

        let out_path = dest.join(&name);

        if entry.is_dir() {
            std::fs::create_dir_all(&out_path).map_err(|e| e.to_string())?;
        } else {
            if let Some(parent) = out_path.parent() {
                std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
            }
            let mut buf = Vec::new();
            std::io::Read::read_to_end(&mut entry, &mut buf).map_err(|e| format!("{e}"))?;
            std::fs::write(&out_path, &buf).map_err(|e| e.to_string())?;
        }
    }

    Ok(())
}
