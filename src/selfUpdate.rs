use crate::{logger, net};
use std::path::{Path, PathBuf};

const GITHUB_API: &str = "https://api.github.com/repos/eddmpython/dartlab-desktop/releases/latest";
const CURRENT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn current_version() -> &'static str {
    CURRENT_VERSION
}

pub fn cleanup_old() {
    let current = std::env::current_exe().ok();
    if let Some(ref exe) = current {
        cleanup_stale_file(&exe.with_extension("exe.old"));
        cleanup_stale_file(&exe.with_extension("exe.new"));
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
    logger::log(&format!(
        "런처 새 버전 발견: {} (현재: {current})",
        release.tag
    ));

    Some(SelfUpdateInfo {
        version: release.tag.trim_start_matches('v').to_string(),
        download_url: url,
    })
}

pub fn apply_update(info: &SelfUpdateInfo) -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let new_exe = current_exe.with_extension("exe.new");
    let old_exe = current_exe.with_extension("exe.old");

    cleanup_stale_file(&new_exe);
    cleanup_stale_file(&old_exe);
    if old_exe.exists() {
        return Err(format!(
            "이전 업데이트 잔여 파일이 남아 있습니다: {}",
            old_exe.display()
        ));
    }

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

pub fn relaunch_updated_exe() -> Result<(), String> {
    let current_exe = std::env::current_exe().map_err(|e| e.to_string())?;
    std::process::Command::new(&current_exe)
        .spawn()
        .map_err(|e| format!("업데이트된 런처 재실행 실패: {e}"))?;
    logger::log("업데이트된 런처 재실행");
    Ok(())
}

struct ReleaseInfo {
    tag: String,
    asset_url: Option<String>,
}

fn get_latest_release() -> Result<ReleaseInfo, String> {
    let body = net::get_text(GITHUB_API, &[("User-Agent", "dartlab-desktop")])?;

    let json: serde_json::Value = serde_json::from_str(&body).map_err(|e| e.to_string())?;

    let tag = json["tag_name"].as_str().ok_or("No tag_name")?.to_string();

    let asset_url = json["assets"].as_array().and_then(|assets| {
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
    net::download_to_file(url, dest, &[("User-Agent", "dartlab-desktop")], None)
}

fn cleanup_stale_file(path: &Path) {
    if !path.exists() {
        return;
    }

    for attempt in 0..3 {
        match std::fs::remove_file(path) {
            Ok(()) => {
                logger::log(&format!("업데이트 잔여 파일 정리: {}", path.display()));
                return;
            }
            Err(e) if attempt < 2 => {
                std::thread::sleep(std::time::Duration::from_millis(300));
                logger::log(&format!(
                    "업데이트 잔여 파일 정리 재시도 ({}/3): {} ({e})",
                    attempt + 2,
                    path.display()
                ));
            }
            Err(e) => {
                logger::log(&format!(
                    "업데이트 잔여 파일 정리 실패: {} ({e})",
                    path.display()
                ));
            }
        }
    }
}
