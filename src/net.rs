use crate::logger;
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::process::Command;

const CREATE_NO_WINDOW: u32 = 0x08000000;

pub fn get_text(url: &str, headers: &[(&str, &str)]) -> Result<String, String> {
    match get_text_ureq(url, headers) {
        Ok(body) => Ok(body),
        Err(err) if should_fallback(url) => {
            logger::log(&format!("HTTPS 요청 PowerShell fallback: {url} ({err})"));
            get_text_powershell(url, headers)
                .map_err(|ps_err| format!("{err}; PowerShell fallback failed: {ps_err}"))
        }
        Err(err) => Err(err),
    }
}

pub fn download_to_file(
    url: &str,
    dest: &Path,
    headers: &[(&str, &str)],
    limit: Option<u64>,
) -> Result<(), String> {
    match download_to_file_ureq(url, dest, headers, limit) {
        Ok(()) => Ok(()),
        Err(err) if should_fallback(url) => {
            logger::log(&format!(
                "HTTPS 다운로드 PowerShell fallback: {url} ({err})"
            ));
            download_to_file_powershell(url, dest, headers)
                .map_err(|ps_err| format!("{err}; PowerShell fallback failed: {ps_err}"))
        }
        Err(err) => Err(err),
    }
}

fn get_text_ureq(url: &str, headers: &[(&str, &str)]) -> Result<String, String> {
    let mut req = ureq::get(url);
    for (key, value) in headers {
        req = req.header(*key, *value);
    }

    let resp = req.call().map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {status}"));
    }

    resp.into_body().read_to_string().map_err(|e| e.to_string())
}

fn download_to_file_ureq(
    url: &str,
    dest: &Path,
    headers: &[(&str, &str)],
    limit: Option<u64>,
) -> Result<(), String> {
    let mut req = ureq::get(url);
    for (key, value) in headers {
        req = req.header(*key, *value);
    }

    let resp = req.call().map_err(|e| e.to_string())?;
    let status = resp.status().as_u16();
    if !(200..300).contains(&status) {
        return Err(format!("HTTP {status}"));
    }

    let mut body = resp.into_body();
    let bytes = match limit {
        Some(limit) => body
            .with_config()
            .limit(limit)
            .read_to_vec()
            .map_err(|e| e.to_string())?,
        None => body.read_to_vec().map_err(|e| e.to_string())?,
    };

    std::fs::write(dest, &bytes).map_err(|e| e.to_string())
}

fn get_text_powershell(url: &str, headers: &[(&str, &str)]) -> Result<String, String> {
    let command = format!(
        "$ProgressPreference='SilentlyContinue'; \
         [Console]::OutputEncoding = [System.Text.Encoding]::UTF8; \
         (Invoke-WebRequest -Uri '{}'{}).Content",
        escape_powershell(url),
        powershell_headers_arg(headers),
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NoLogo", "-Command", &command])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("PowerShell 실행 실패: {e}"))?;

    if !output.status.success() {
        return Err(powershell_stderr(&output.stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

fn download_to_file_powershell(
    url: &str,
    dest: &Path,
    headers: &[(&str, &str)],
) -> Result<(), String> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }

    let command = format!(
        "$ProgressPreference='SilentlyContinue'; \
         Invoke-WebRequest -Uri '{}'{} -OutFile '{}'",
        escape_powershell(url),
        powershell_headers_arg(headers),
        escape_powershell(&dest.to_string_lossy()),
    );

    let output = Command::new("powershell")
        .args(["-NoProfile", "-NoLogo", "-Command", &command])
        .creation_flags(CREATE_NO_WINDOW)
        .output()
        .map_err(|e| format!("PowerShell 실행 실패: {e}"))?;

    if !output.status.success() {
        return Err(powershell_stderr(&output.stderr));
    }

    Ok(())
}

fn should_fallback(url: &str) -> bool {
    url.starts_with("https://")
}

fn powershell_headers_arg(headers: &[(&str, &str)]) -> String {
    if headers.is_empty() {
        return String::new();
    }

    let pairs = headers
        .iter()
        .map(|(key, value)| {
            format!(
                "'{}'='{}'",
                escape_powershell(key),
                escape_powershell(value)
            )
        })
        .collect::<Vec<_>>()
        .join("; ");

    format!(" -Headers @{{ {pairs} }}")
}

fn escape_powershell(value: &str) -> String {
    value.replace('\'', "''")
}

fn powershell_stderr(stderr: &[u8]) -> String {
    let text = String::from_utf8_lossy(stderr).trim().to_string();
    if text.is_empty() {
        "PowerShell 다운로드 실패".to_string()
    } else {
        text
    }
}
