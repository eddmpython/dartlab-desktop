use crate::{logger, paths};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn ensure_shortcuts() -> Result<(), String> {
    let current_exe =
        std::env::current_exe().map_err(|e| format!("현재 exe 경로 확인 실패: {e}"))?;
    let work_dir = current_exe
        .parent()
        .ok_or("현재 exe 작업 디렉터리를 확인할 수 없습니다".to_string())?;

    let mut errors = Vec::new();

    match paths::desktop_shortcut() {
        Some(path) => {
            if let Err(e) = create_shortcut(&path, &current_exe, work_dir, "DartLab Desktop") {
                errors.push(format!("바탕화면 바로가기 생성 실패: {e}"));
            }
        }
        None => errors.push("바탕화면 경로를 찾을 수 없습니다".to_string()),
    }

    match paths::start_menu_shortcut() {
        Some(path) => {
            if let Err(e) = create_shortcut(&path, &current_exe, work_dir, "DartLab Desktop") {
                errors.push(format!("시작 메뉴 바로가기 생성 실패: {e}"));
            }
        }
        None => errors.push("시작 메뉴 경로를 찾을 수 없습니다".to_string()),
    }

    if errors.is_empty() {
        logger::log("Desktop/Start Menu 바로가기 동기화 완료");
        Ok(())
    } else {
        Err(errors.join("\n"))
    }
}

fn create_shortcut(
    shortcut_path: &Path,
    target_path: &Path,
    working_dir: &Path,
    description: &str,
) -> Result<(), String> {
    if let Some(parent) = shortcut_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let shortcut = shortcut_path.to_string_lossy();
    let target = target_path.to_string_lossy();
    let work_dir = working_dir.to_string_lossy();
    let icon = format!("{target},0");

    logger::log(&format!("바로가기 생성/복구: {}", shortcut_path.display()));

    let script = format!(
        "$shell = New-Object -ComObject WScript.Shell; \
         $shortcut = $shell.CreateShortcut('{}'); \
         $shortcut.TargetPath = '{}'; \
         $shortcut.WorkingDirectory = '{}'; \
         $shortcut.IconLocation = '{}'; \
         $shortcut.Description = '{}'; \
         $shortcut.Save()",
        escape_powershell(&shortcut),
        escape_powershell(&target),
        escape_powershell(&work_dir),
        escape_powershell(&icon),
        escape_powershell(description),
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NoLogo", "-Command", &script])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("PowerShell 실행 실패: {e}"))?;

    if !output.status.success() {
        return Err(command_error(&output.stdout, &output.stderr));
    }

    Ok(())
}

fn escape_powershell(value: &str) -> String {
    value.replace('\'', "''")
}

fn command_error(stdout: &[u8], stderr: &[u8]) -> String {
    let stderr_text = String::from_utf8_lossy(stderr).trim().to_string();
    if !stderr_text.is_empty() {
        return stderr_text;
    }

    let stdout_text = String::from_utf8_lossy(stdout).trim().to_string();
    if stdout_text.is_empty() {
        "바로가기 생성 실패".to_string()
    } else {
        stdout_text
    }
}
