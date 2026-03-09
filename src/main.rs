#![windows_subsystem = "windows"]

mod setup;
mod runner;
mod updater;
mod ollama;
mod paths;
mod ui;
#[allow(non_snake_case)]
mod selfUpdate;

use std::sync::mpsc;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::WindowBuilder;
use wry::WebViewBuilder;

#[derive(Debug)]
enum AppEvent {
    Log(String),
    Ready,
    Error(String),
}

const SETUP_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
<meta charset="utf-8">
<style>
  * { margin: 0; padding: 0; box-sizing: border-box; }
  body {
    font-family: -apple-system, 'Segoe UI', sans-serif;
    background: #050811;
    color: #f1f5f9;
    display: flex;
    flex-direction: column;
    align-items: center;
    justify-content: center;
    height: 100vh;
    overflow: hidden;
  }
  h1 { font-size: 28px; font-weight: 700; margin-bottom: 8px; }
  .sub { color: #94a3b8; font-size: 14px; margin-bottom: 40px; }
  #log {
    width: 400px;
    font-family: 'Consolas', 'Courier New', monospace;
    font-size: 13px;
    color: #94a3b8;
    line-height: 1.8;
    text-align: center;
  }
  .line { opacity: 0; animation: fadeIn 0.3s forwards; }
  .line.ok { color: #34d399; }
  .line.err { color: #ea4647; }
  .spinner {
    display: inline-block;
    width: 12px; height: 12px;
    border: 2px solid #64748b;
    border-top-color: #ea4647;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
    margin-right: 8px;
    vertical-align: middle;
  }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes fadeIn { to { opacity: 1; } }
</style>
</head>
<body>
  <h1>DartLab</h1>
  <p class="sub">AI 기업분석 준비 중</p>
  <div id="log"></div>
  <script>
    function addLog(msg, cls) {
      const el = document.getElementById('log');
      const div = document.createElement('div');
      div.className = 'line' + (cls ? ' ' + cls : '');
      div.innerHTML = msg;
      el.appendChild(div);
      el.scrollTop = el.scrollHeight;
    }
  </script>
</body>
</html>"#;

fn main() {
    selfUpdate::cleanup_old();
    selfUpdate::check_and_update();

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let window = WindowBuilder::new()
        .with_title("DartLab")
        .with_inner_size(tao::dpi::LogicalSize::new(1200.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0))
        .build(&event_loop)
        .expect("Failed to create window");

    let webview = WebViewBuilder::new()
        .with_html(SETUP_HTML)
        .build(&window)
        .expect("Failed to create webview");

    let (tx, rx) = mpsc::channel::<AppEvent>();

    let proxy_clone = proxy.clone();
    std::thread::spawn(move || {
        run_setup(tx, proxy_clone);
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        if let Ok(app_event) = rx.try_recv() {
            match app_event {
                AppEvent::Log(msg) => {
                    let escaped = msg.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n");
                    let js = format!("addLog('{escaped}', '')");
                    webview.evaluate_script(&js).ok();
                }
                AppEvent::Ready => {
                    let _ = webview.load_url("http://localhost:8400");
                }
                AppEvent::Error(msg) => {
                    let escaped = msg.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n");
                    let js = format!("addLog('{escaped}', 'err')");
                    webview.evaluate_script(&js).ok();
                }
            }
        }

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                runner::stop_server();
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

fn run_setup(tx: mpsc::Sender<AppEvent>, proxy: tao::event_loop::EventLoopProxy<AppEvent>) {
    let send = |msg: &str| {
        let _ = tx.send(AppEvent::Log(msg.to_string()));
        let _ = proxy.send_event(AppEvent::Log(msg.to_string()));
    };
    let send_ok = |msg: &str| {
        let formatted = format!("<span class=\"ok\">+ {msg}</span>");
        let _ = tx.send(AppEvent::Log(formatted.clone()));
        let _ = proxy.send_event(AppEvent::Log(formatted));
    };
    let send_err = |msg: &str| {
        let _ = tx.send(AppEvent::Error(msg.to_string()));
        let _ = proxy.send_event(AppEvent::Error(msg.to_string()));
    };

    let app_dir = paths::app_dir();
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir).ok();
    }

    send("<span class=\"spinner\"></span> [1/4] uv 설치 확인 중...");
    if let Err(e) = setup::ensure_uv(&app_dir) {
        send_err(&format!("uv 설치 실패: {e}"));
        return;
    }
    send_ok("uv 준비 완료");

    send("<span class=\"spinner\"></span> [2/4] Python 환경 확인 중...");
    if let Err(e) = setup::ensure_dartlab(&app_dir) {
        send_err(&format!("DartLab 설치 실패: {e}"));
        return;
    }
    send_ok("DartLab 설치 완료");

    match updater::check_update(&app_dir) {
        Ok(Some(latest)) => {
            send(&format!("<span class=\"spinner\"></span> 새 버전 발견: {latest} — 업데이트 중..."));
            if let Err(e) = updater::do_update(&app_dir) {
                send(&format!("! 업데이트 실패 (기존 버전으로 진행): {e}"));
            } else {
                send_ok("업데이트 완료");
            }
        }
        _ => {}
    }

    send("<span class=\"spinner\"></span> [3/4] Ollama 확인 중...");
    if let Err(e) = ollama::ensure_ollama() {
        send(&format!("! Ollama 건너뜀: {e}"));
    } else {
        send_ok("Ollama 준비 완료");
    }

    send("<span class=\"spinner\"></span> [4/4] 서버 시작 중...");
    if let Err(e) = runner::start_server(&app_dir) {
        send_err(&format!("서버 시작 실패: {e}"));
        return;
    }

    send("<span class=\"spinner\"></span> 서버 응답 대기 중...");
    if runner::wait_for_server(15) {
        send_ok("준비 완료!");
        std::thread::sleep(std::time::Duration::from_millis(500));
        let _ = tx.send(AppEvent::Ready);
        let _ = proxy.send_event(AppEvent::Ready);
    } else {
        send_err("서버 응답 시간 초과");
    }
}
