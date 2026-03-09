use std::io::{self, Write};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn print_banner() {
    println!();
    println!("  ╔══════════════════════════════════╗");
    println!("  ║         DartLab Desktop           ║");
    println!("  ║    AI 기업분석 — 더블클릭 실행     ║");
    println!("  ╚══════════════════════════════════╝");
    println!("  v{VERSION}");
    println!();
}

pub fn print_step(step: u8, total: u8, msg: &str) {
    println!("  [{step}/{total}] {msg}");
}

pub fn print_info(msg: &str) {
    println!("  ℹ {msg}");
}

pub fn print_ok(msg: &str) {
    println!("  ✓ {msg}");
}

pub fn print_warn(msg: &str) {
    println!("  ⚠ {msg}");
}

pub fn print_error(msg: &str) {
    eprintln!("  ✗ {msg}");
}

pub fn wait_for_enter() {
    print!("\n  Enter를 누르면 종료합니다...");
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
}
