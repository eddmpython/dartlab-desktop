mod setup;
mod runner;
mod updater;
mod ollama;
mod paths;
mod ui;

use std::process::ExitCode;

fn main() -> ExitCode {
    ui::print_banner();

    let app_dir = paths::app_dir();
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir).expect("Failed to create app directory");
    }

    if let Err(e) = setup::ensure_uv(&app_dir) {
        ui::print_error(&format!("uv 설치 실패: {e}"));
        ui::wait_for_enter();
        return ExitCode::FAILURE;
    }

    if let Err(e) = setup::ensure_dartlab(&app_dir) {
        ui::print_error(&format!("DartLab 설치 실패: {e}"));
        ui::wait_for_enter();
        return ExitCode::FAILURE;
    }

    match updater::check_update(&app_dir) {
        Ok(Some(latest)) => {
            ui::print_info(&format!("새 버전 발견: {latest}"));
            if let Err(e) = updater::do_update(&app_dir) {
                ui::print_warn(&format!("업데이트 실패 (기존 버전으로 실행): {e}"));
            }
        }
        Ok(None) => {}
        Err(_) => {}
    }

    if let Err(e) = ollama::ensure_ollama() {
        ui::print_warn(&format!("Ollama 설정 건너뜀: {e}"));
    }

    ui::print_info("DartLab AI 시작 중...");

    match runner::run_dartlab(&app_dir) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            ui::print_error(&format!("실행 실패: {e}"));
            ui::wait_for_enter();
            ExitCode::FAILURE
        }
    }
}
