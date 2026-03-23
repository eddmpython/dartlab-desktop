use crate::{logger, net, paths};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

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
            .args([
                "pip",
                "install",
                "dartlab[ai,llm]",
                "--python",
                python.to_str().unwrap(),
            ])
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

pub fn ensure_ui_build(app_dir: &Path) -> Result<(), String> {
    let ui_dir = paths::dartlab_ui_dir(app_dir);
    let build_dir = ui_dir.join("build");
    let version_file = build_dir.join(".dartlab_ui_version");

    let current_ver = get_dartlab_version(app_dir);

    if build_dir.exists() && build_dir.join("index.html").exists() {
        if let Some(ref ver) = current_ver {
            if let Ok(saved) = std::fs::read_to_string(&version_file) {
                if saved.trim() == ver.trim() {
                    return Ok(());
                }
                logger::log(&format!(
                    "dartlab 버전 변경 감지 ({} → {ver}) — UI 재다운로드",
                    saved.trim()
                ));
            } else {
                return Ok(());
            }
        } else {
            return Ok(());
        }
    }

    logger::log("UI 빌드 파일 다운로드 중...");

    let zip_url = "https://github.com/eddmpython/dartlab/archive/refs/heads/master.zip";
    let zip_path = app_dir.join("dartlab-ui.zip");
    download_file(zip_url, &zip_path)?;

    let extract_dir = app_dir.join("_dartlab_ui_tmp");
    if extract_dir.exists() {
        std::fs::remove_dir_all(&extract_dir).ok();
    }
    std::fs::create_dir_all(&extract_dir).map_err(|e| e.to_string())?;
    extract_zip(&zip_path, &extract_dir)?;
    std::fs::remove_file(&zip_path).ok();

    let src_build = find_build_dir(&extract_dir)
        .ok_or("다운로드한 레포에서 ui/build 디렉토리를 찾을 수 없습니다")?;

    std::fs::create_dir_all(&ui_dir).map_err(|e| e.to_string())?;

    if build_dir.exists() {
        std::fs::remove_dir_all(&build_dir).ok();
    }
    copy_dir_all(&src_build, &build_dir)?;

    std::fs::remove_dir_all(&extract_dir).ok();

    if !build_dir.join("index.html").exists() {
        return Err("UI 빌드 파일 복사 완료되었으나 index.html이 없습니다".into());
    }

    if let Some(ref ver) = current_ver {
        std::fs::write(&version_file, ver).ok();
    }

    logger::log("UI 빌드 파일 설치 완료");
    Ok(())
}

fn get_dartlab_version(app_dir: &Path) -> Option<String> {
    let python = paths::python_bin(app_dir);
    let output = Command::new(&python)
        .args(["-c", "import dartlab; print(dartlab.__version__)"])
        .current_dir(app_dir)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if ver.is_empty() { None } else { Some(ver) }
}

fn find_build_dir(extract_dir: &Path) -> Option<std::path::PathBuf> {
    if let Ok(entries) = std::fs::read_dir(extract_dir) {
        for entry in entries.flatten() {
            let candidate = entry
                .path()
                .join("src")
                .join("dartlab")
                .join("ui")
                .join("build");
            if candidate.join("index.html").exists() {
                return Some(candidate);
            }
        }
    }
    None
}

fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dst).map_err(|e| e.to_string())?;
    for entry in std::fs::read_dir(src).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        let ty = entry.file_type().map_err(|e| e.to_string())?;
        let dst_path = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dst_path)?;
        } else {
            std::fs::copy(entry.path(), &dst_path).map_err(|e| e.to_string())?;
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
    download_file_with_limit(url, dest, 512 * 1024 * 1024)
}

fn download_file_with_limit(url: &str, dest: &Path, limit: u64) -> Result<(), String> {
    let mut last_err = String::new();
    for attempt in 0..3 {
        if attempt > 0 {
            logger::log(&format!("다운로드 재시도 ({}/3): {url}", attempt + 1));
            std::thread::sleep(std::time::Duration::from_secs(3));
        }

        match net::download_to_file(url, dest, &[("User-Agent", "dartlab-desktop")], Some(limit)) {
            Ok(()) => {
                return Ok(());
            }
            Err(e) => {
                last_err = e;
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
