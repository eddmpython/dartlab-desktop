use std::path::Path;
use std::process::Command;
use crate::{paths, ui};

const PORT: u16 = 8400;
const URL: &str = "http://localhost:8400";

pub fn run_dartlab(app_dir: &Path) -> Result<(), String> {
    let python = paths::python_bin(app_dir);

    ui::print_info(&format!("브라우저에서 {URL} 을 엽니다"));
    ui::print_info("종료하려면 이 창을 닫으세요");
    println!();

    std::thread::spawn(|| {
        std::thread::sleep(std::time::Duration::from_secs(3));
        open::that(URL).ok();
    });

    let status = Command::new(&python)
        .args([
            "-m",
            "dartlab",
            "ai",
            "--port",
            &PORT.to_string(),
        ])
        .current_dir(app_dir)
        .status()
        .map_err(|e| e.to_string())?;

    if !status.success() {
        return Err("dartlab ai exited with error".into());
    }

    Ok(())
}
