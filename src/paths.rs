use std::path::PathBuf;

pub fn app_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("C:\\Users\\Public"))
        .join("DartLab")
}

pub fn uv_bin(app_dir: &std::path::Path) -> PathBuf {
    app_dir.join("uv").join("uv.exe")
}

pub fn venv_dir(app_dir: &std::path::Path) -> PathBuf {
    app_dir.join(".venv")
}

pub fn python_bin(app_dir: &std::path::Path) -> PathBuf {
    venv_dir(app_dir).join("Scripts").join("python.exe")
}
