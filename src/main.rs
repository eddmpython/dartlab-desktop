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
use tao::window::{Icon, WindowBuilder};
use wry::{WebViewBuilder, Rect};

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
  .avatar {
    width: 80px; height: 80px;
    border-radius: 50%;
    margin-bottom: 20px;
    opacity: 0;
    animation: fadeIn 0.5s 0.2s forwards;
  }
  h1 { font-size: 26px; font-weight: 700; margin-bottom: 6px; }
  .sub { color: #94a3b8; font-size: 13px; margin-bottom: 36px; }
  .bar-wrap {
    width: 280px; height: 4px;
    background: #1e2433;
    border-radius: 2px;
    overflow: hidden;
    margin-bottom: 16px;
  }
  .bar {
    height: 100%; width: 0%;
    background: linear-gradient(90deg, #ea4647, #fb923c);
    border-radius: 2px;
    transition: width 0.4s ease;
  }
  #status {
    font-size: 13px;
    color: #64748b;
    min-height: 20px;
  }
  #error {
    color: #ea4647;
    font-size: 12px;
    margin-top: 8px;
    max-width: 400px;
    text-align: center;
    display: none;
  }
  @keyframes fadeIn { to { opacity: 1; } }
</style>
</head>
<body>
  <img class="avatar" src="https://eddmpython.github.io/dartlab/avatar-analyze.png" alt="">
  <h1>DartLab</h1>
  <p class="sub">AI 기업분석 준비 중</p>
  <div class="bar-wrap"><div class="bar" id="bar"></div></div>
  <div id="status"></div>
  <div id="error"></div>
  <script>
    function setProgress(pct, label) {
      document.getElementById('bar').style.width = pct + '%';
      document.getElementById('status').textContent = label;
    }
    function setError(msg) {
      var el = document.getElementById('error');
      el.textContent = msg;
      el.style.display = 'block';
    }
  </script>
</body>
</html>"#;

fn main() {
    selfUpdate::cleanup_old();
    selfUpdate::check_and_update();

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let icon = load_window_icon();

    let mut wb = WindowBuilder::new()
        .with_title("DartLab")
        .with_inner_size(tao::dpi::LogicalSize::new(1200.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0))
        .with_focused(true);

    if let Some(ic) = icon {
        wb = wb.with_window_icon(Some(ic));
    }

    let window = wb.build(&event_loop).expect("Failed to create window");
    window.set_focus();

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
                    let _ = webview.load_url("http://127.0.0.1:8400");
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
                event: WindowEvent::Resized(size),
                ..
            } => {
                let _ = webview.set_bounds(Rect {
                    position: tao::dpi::LogicalPosition::new(0, 0).into(),
                    size: tao::dpi::LogicalSize::new(size.width, size.height).into(),
                });
            }
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

    let progress = |pct: u32, label: &str| {
        let escaped = label.replace('\'', "\\'");
        js(&format!("setProgress({pct},'{escaped}')"));
    };

    let fail = |msg: &str| {
        let escaped = msg.replace('\'', "\\'");
        js(&format!("setError('{escaped}')"));
    };

    let app_dir = paths::app_dir();
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir).ok();
    }

    progress(10, "환경 준비 중...");
    if let Err(e) = setup::ensure_uv(&app_dir) {
        fail(&format!("uv 설치 실패: {e}"));
        return;
    }

    progress(30, "DartLab 설치 확인 중...");
    if let Err(e) = setup::ensure_dartlab(&app_dir) {
        fail(&format!("DartLab 설치 실패: {e}"));
        return;
    }

    progress(50, "업데이트 확인 중...");
    match updater::check_update(&app_dir) {
        Ok(Some(latest)) => {
            progress(55, &format!("v{latest} 업데이트 중..."));
            if let Err(_e) = updater::do_update(&app_dir) {}
        }
        _ => {}
    }

    progress(70, "Ollama 확인 중...");
    let _ = ollama::ensure_ollama();

    progress(85, "서버 시작 중...");
    if let Err(e) = runner::start_server(&app_dir) {
        fail(&format!("서버 시작 실패: {e}"));
        return;
    }

    progress(90, "서버 응답 대기 중...");
    match runner::wait_for_server(30) {
        Ok(()) => {
            progress(100, "준비 완료!");
            std::thread::sleep(std::time::Duration::from_millis(300));
            let _ = tx.send(AppEvent::Ready);
            let _ = proxy.send_event(AppEvent::Ready);
        }
        Err(e) => {
            fail(&e);
        }
    }
}

fn load_window_icon() -> Option<Icon> {
    let ico_bytes = include_bytes!("../assets/icon.ico");
    let reader = image::ImageReader::with_format(
        std::io::Cursor::new(ico_bytes),
        image::ImageFormat::Ico,
    );
    let img = reader.decode().ok()?;
    let rgba = img.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Icon::from_rgba(rgba.into_raw(), w, h).ok()
}
