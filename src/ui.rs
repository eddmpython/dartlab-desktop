use std::io::{self, Write};

const VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn print_banner() {
    println!();
    println!("  DartLab Desktop v{VERSION}");
    println!("  -------------------------");
    println!();
}

pub fn print_step(step: u8, total: u8, msg: &str) {
    println!("  [{step}/{total}] {msg}");
}

pub fn print_info(msg: &str) {
    println!("  > {msg}");
}

pub fn print_ok(msg: &str) {
    println!("  + {msg}");
}

pub fn print_warn(msg: &str) {
    println!("  ! {msg}");
}

pub fn print_error(msg: &str) {
    eprintln!("  x {msg}");
}

pub fn wait_for_enter() {
    print!("\n  Enter to exit...");
    io::stdout().flush().ok();
    let mut buf = String::new();
    io::stdin().read_line(&mut buf).ok();
}
