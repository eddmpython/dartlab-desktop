use crate::{net, paths};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const PYPI_URL: &str = "https://pypi.org/pypi/dartlab/json";
const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn check_update(app_dir: &Path) -> Result<Option<String>, String> {
    let local_ver = get_local_version(app_dir)?;
    let latest_ver = get_pypi_version()?;

    if latest_ver != local_ver {
        Ok(Some(latest_ver))
    } else {
        Ok(None)
    }
}

pub fn do_update(app_dir: &Path) -> Result<(), String> {
    let uv = paths::uv_bin(app_dir);
    let python = paths::python_bin(app_dir);

    let status = Command::new(&uv)
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
        .status()
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("uv pip install --upgrade failed".into());
    }

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
