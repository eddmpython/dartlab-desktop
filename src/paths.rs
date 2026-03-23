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

pub fn dartlab_bin(app_dir: &std::path::Path) -> PathBuf {
    venv_dir(app_dir).join("Scripts").join("dartlab.exe")
}

pub fn dartlab_ui_dir(app_dir: &std::path::Path) -> PathBuf {
    venv_dir(app_dir)
        .join("Lib")
        .join("site-packages")
        .join("dartlab")
        .join("ui")
}

pub fn desktop_shortcut() -> Option<PathBuf> {
    dirs::desktop_dir().map(|dir| dir.join("DartLab.lnk"))
}

pub fn start_menu_shortcut() -> Option<PathBuf> {
    std::env::var_os("APPDATA").map(|appdata| {
        PathBuf::from(appdata)
            .join("Microsoft")
            .join("Windows")
            .join("Start Menu")
            .join("Programs")
            .join("DartLab.lnk")
    })
}
