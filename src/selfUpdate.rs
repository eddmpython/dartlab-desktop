use std::path::PathBuf;
use crate::logger;

const GITHUB_API: &str = "https://api.github.com/repos/eddmpython/dartlab-desktop/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

pub fn cleanup_old() {
    let current = std::env::current_exe().ok();
    if let Some(ref exe) = current {
        let old = exe.with_extension("exe.old");
        if old.exists() {
            std::fs::remove_file(&old).ok();
        }
    }
}

pub struct SelfUpdateInfo {
    pub version: String,
    pub download_url: String,
}

pub fn check_update() -> Option<SelfUpdateInfo> {
    let release = get_latest_release().ok()?;

    let current = format!("v{CURRENT_VERSION}");
    if release.tag == current {
        return None;
    }

    let url = release.asset_url?;
    logger::log(&format!("런처 새 버전 발견: {} (현재: {current})", release.tag));

    Some(SelfUpdateInfo {
        version: release.tag.trim_start_matches('v').to_string(),
        download_url: url,
    })
}

pub fn apply_update(info: &SelfUpdateInfo) -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let new_exe = current_exe.with_extension("exe.new");
    let old_exe = current_exe.with_extension("exe.old");

    logger::log(&format!("런처 업데이트 다운로드 시작: v{}", info.version));

    if let Err(e) = download_file(&info.download_url, &new_exe) {
        std::fs::remove_file(&new_exe).ok();
        logger::log(&format!("런처 다운로드 실패: {e}"));
        return Err(e);
    }

    if std::fs::rename(&current_exe, &old_exe).is_err() {
        std::fs::remove_file(&new_exe).ok();
        return Err("현재 exe 이름 변경 실패".into());
    }

    if std::fs::rename(&new_exe, &current_exe).is_err() {
        std::fs::rename(&old_exe, &current_exe).ok();
        return Err("새 exe 이름 변경 실패".into());
    }

    logger::log(&format!("런처 업데이트 완료: v{}", info.version));
    Ok(())
}

struct ReleaseInfo {
    tag: String,
    asset_url: Option<String>,
}

fn get_latest_release() -> Result<ReleaseInfo, String> {
    let resp = ureq::get(GITHUB_API)
        .header("User-Agent", "dartlab-desktop")
        .call()
        .map_err(|e| e.to_string())?;

    let status = resp.status().as_u16();
    if status < 200 || status >= 300 {
        return Err(format!("HTTP {}", status));
    }

    let body = resp.into_body()
        .read_to_string()
        .map_err(|e| e.to_string())?;

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
    let resp = ureq::get(url)
        .header("User-Agent", "dartlab-desktop")
        .call()
        .map_err(|e| format!("Download failed: {e}"))?;

    let status = resp.status().as_u16();
    if status < 200 || status >= 300 {
        return Err(format!("HTTP {}", status));
    }

    let bytes = resp.into_body().read_to_vec().map_err(|e| e.to_string())?;
    std::fs::write(dest, &bytes).map_err(|e| e.to_string())?;
    Ok(())
}
