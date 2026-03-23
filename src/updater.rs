use crate::{logger, net, paths};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const PYPI_URL: &str = "https://pypi.org/pypi/dartlab/json";
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn check_update(app_dir: &Path) -> Result<Option<String>, String> {
    let local_ver = get_local_version(app_dir)?;
    let latest_ver = get_pypi_version()?;

    logger::log(&format!("DartLab 로컬 버전: {local_ver}"));
    logger::log(&format!("DartLab PyPI 최신 버전: {latest_ver}"));

    if latest_ver != local_ver {
        logger::log(&format!(
            "DartLab 업데이트 필요: {local_ver} -> {latest_ver}"
        ));
        Ok(Some(latest_ver))
    } else {
        logger::log("DartLab 최신 버전 사용 중");
        Ok(None)
    }
}

pub fn do_update(app_dir: &Path) -> Result<(), String> {
    let uv = paths::uv_bin(app_dir);
    let python = paths::python_bin(app_dir);

    let output = Command::new(&uv)
        .args([
            "pip",
            "install",
            "--upgrade",
            "dartlab[ai,llm]",
            "--python",
            python.to_str().unwrap(),
        ])
        .current_dir(app_dir)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        logger::log(&format!("uv pip install --upgrade stderr: {stderr}"));
        return Err(format!("uv pip install --upgrade failed: {stderr}"));
    }

    logger::log("DartLab 패키지 업데이트 완료");
    Ok(())
}

fn get_local_version(app_dir: &Path) -> Result<String, String> {
    let python = paths::python_bin(app_dir);

    let output = Command::new(&python)
        .args(["-c", "import dartlab; print(dartlab.__version__)"])
        .current_dir(app_dir)
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| e.to_string())?;

    if !output.status.success() {
        return Err("Failed to get local version".into());
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn get_pypi_version() -> Result<String, String> {
    let body = net::get_text(PYPI_URL, &[("Accept", "application/json")])
        .map_err(|e| format!("PyPI request failed: {e}"))?;

    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;
    let version = json["info"]["version"]
        .as_str()
        .ok_or("No version in PyPI response")?
        .to_string();

    Ok(version)
}
