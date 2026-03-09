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
#[allow(dead_code)]
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
    width: 420px;
    font-family: 'Consolas', 'Courier New', monospace;
    font-size: 13px;
    color: #94a3b8;
    line-height: 2;
    text-align: left;
  }
  .step { opacity: 0; animation: fadeIn 0.3s forwards; display: flex; align-items: center; gap: 10px; }
  .icon { width: 18px; text-align: center; flex-shrink: 0; }
  .spinner {
    display: inline-block;
    width: 14px; height: 14px;
    border: 2px solid #64748b;
    border-top-color: #ea4647;
    border-radius: 50%;
    animation: spin 0.8s linear infinite;
  }
  .check { color: #34d399; font-weight: bold; }
  .fail { color: #ea4647; font-weight: bold; }
  .detail { color: #64748b; font-size: 12px; margin-left: 28px; }
  @keyframes spin { to { transform: rotate(360deg); } }
  @keyframes fadeIn { to { opacity: 1; } }
</style>
</head>
<body>
  <h1>DartLab</h1>
  <p class="sub">AI 기업분석 준비 중</p>
  <div id="log"></div>
  <script>
    let currentStep = null;
    function startStep(label) {
      const el = document.getElementById('log');
      const div = document.createElement('div');
      div.className = 'step';
      div.innerHTML = '<span class="icon"><span class="spinner"></span></span><span>' + label + '</span>';
      el.appendChild(div);
      currentStep = div;
    }
    function completeStep() {
      if (!currentStep) return;
      currentStep.querySelector('.icon').innerHTML = '<span class="check">&#10003;</span>';
      currentStep = null;
    }
    function failStep(msg) {
      if (!currentStep) return;
      currentStep.querySelector('.icon').innerHTML = '<span class="fail">&#10007;</span>';
      if (msg) {
        const el = document.getElementById('log');
        const detail = document.createElement('div');
        detail.className = 'detail';
        detail.textContent = msg;
        el.appendChild(detail);
      }
      currentStep = null;
    }
    function addInfo(msg) {
      const el = document.getElementById('log');
      const div = document.createElement('div');
      div.className = 'detail';
      div.textContent = msg;
      el.appendChild(div);
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

        while let Ok(app_event) = rx.try_recv() {
            match app_event {
                AppEvent::Log(script) => {
                    webview.evaluate_script(&script).ok();
                }
                AppEvent::Ready => {
                    let _ = webview.load_url("http://localhost:8400");
                }
                AppEvent::Error(msg) => {
                    let escaped = msg.replace('\\', "\\\\").replace('\'', "\\'").replace('\n', "\\n");
                    let js = format!("failStep('{escaped}')");
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
    let js = |script: &str| {
        let _ = tx.send(AppEvent::Log(script.to_string()));
        let _ = proxy.send_event(AppEvent::Log(script.to_string()));
    };

    let app_dir = paths::app_dir();
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir).ok();
    }

    js("startStep('uv 확인 중...')");
    if let Err(e) = setup::ensure_uv(&app_dir) {
        js(&format!("failStep('uv 설치 실패: {e}')"));
        return;
    }
    js("completeStep()");

    js("startStep('Python + DartLab 확인 중...')");
    if let Err(e) = setup::ensure_dartlab(&app_dir) {
        js(&format!("failStep('DartLab 설치 실패: {e}')"));
        return;
    }
    js("completeStep()");

    match updater::check_update(&app_dir) {
        Ok(Some(latest)) => {
            js(&format!("startStep('새 버전 {latest} 업데이트 중...')"));
            if let Err(e) = updater::do_update(&app_dir) {
                js(&format!("failStep('업데이트 실패: {e}')"));
            } else {
                js("completeStep()");
            }
        }
        _ => {}
    }

    js("startStep('Ollama 확인 중...')");
    if let Err(e) = ollama::ensure_ollama() {
        js(&format!("failStep('{e}')"));
    } else {
        js("completeStep()");
    }

    js("startStep('서버 시작 중...')");
    if let Err(e) = runner::start_server(&app_dir) {
        js(&format!("failStep('서버 시작 실패: {e}')"));
        return;
    }
    js("completeStep()");

    js("startStep('서버 응답 대기 중...')");
    match runner::wait_for_server(60) {
        Ok(()) => {
            js("completeStep()");
            std::thread::sleep(std::time::Duration::from_millis(500));
            let _ = tx.send(AppEvent::Ready);
            let _ = proxy.send_event(AppEvent::Ready);
        }
        Err(e) => {
            js(&format!("failStep('{e}')"));
        }
    }
}
