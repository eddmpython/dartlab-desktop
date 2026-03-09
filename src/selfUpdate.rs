use std::path::PathBuf;
use crate::ui;

const GITHUB_API: &str = "https://api.github.com/repos/eddmpython/dartlab-desktop/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn cleanup_old() {
    let current = std::env::current_exe().ok();
    if let Some(ref exe) = current {
        let old = exe.with_extension("exe.old");
        if old.exists() {
            std::fs::remove_file(&old).ok();
        }
    }
}

pub fn check_and_update() {
    let latest = match get_latest_release() {
        Ok(r) => r,
        Err(_) => return,
    };

    let current = format!("v{CURRENT_VERSION}");
    if latest.tag == current {
        return;
    }

    ui::print_info(&format!("런처 업데이트 발견: {current} → {}", latest.tag));

    let download_url = match latest.asset_url {
        Some(url) => url,
        None => return,
    };

    let current_exe = match std::env::current_exe() {
        Ok(p) => p,
        Err(_) => return,
    };

    let new_exe = current_exe.with_extension("exe.new");
    let old_exe = current_exe.with_extension("exe.old");

    if let Err(e) = download_file(&download_url, &new_exe) {
        ui::print_warn(&format!("런처 다운로드 실패: {e}"));
        std::fs::remove_file(&new_exe).ok();
        return;
    }

    if std::fs::rename(&current_exe, &old_exe).is_err() {
        ui::print_warn("런처 교체 실패 (권한 부족)");
        std::fs::remove_file(&new_exe).ok();
        return;
    }

    if std::fs::rename(&new_exe, &current_exe).is_err() {
        std::fs::rename(&old_exe, &current_exe).ok();
        ui::print_warn("런처 교체 실패");
        return;
    }

    ui::print_ok(&format!("런처 업데이트 완료 ({})", latest.tag));
    ui::print_info("다음 실행 시 새 버전이 적용됩니다");
}

struct ReleaseInfo {
    tag: String,
    asset_url: Option<String>,
}

fn get_latest_release() -> Result<ReleaseInfo, String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("dartlab-desktop")
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(GITHUB_API).send().map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let body = resp.text().map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    let tag = json["tag_name"]
        .as_str()
        .ok_or("No tag_name")?
        .to_string();

    let asset_url = json["assets"]
        .as_array()
        .and_then(|assets| {
            assets.iter().find_map(|a| {
                let name = a["name"].as_str()?;
                if name == "DartLab.exe" {
                    a["browser_download_url"].as_str().map(|s| s.to_string())
                } else {
                    None
                }
            })
        });

    Ok(ReleaseInfo { tag, asset_url })
}

fn download_file(url: &str, dest: &PathBuf) -> Result<(), String> {
    let client = reqwest::blocking::Client::builder()
        .user_agent("dartlab-desktop")
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client.get(url).send().map_err(|e| format!("Download failed: {e}"))?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    std::fs::write(dest, &bytes).map_err(|e| e.to_string())?;
    Ok(())
}
