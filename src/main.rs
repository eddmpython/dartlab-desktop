#![windows_subsystem = "windows"]

mod logger;
mod net;
mod ollama;
mod paths;
mod runner;
#[allow(non_snake_case)]
mod selfUpdate;
mod setup;
mod shortcuts;
mod state;
mod updater;

use std::process::Command;
use std::sync::mpsc;
use tao::event::{Event, WindowEvent};
use tao::event_loop::{ControlFlow, EventLoopBuilder};
use tao::window::{Icon, WindowBuilder};
use wry::{Rect, WebViewBuilder};

#[derive(Debug)]
enum AppEvent {
    Log(String),
    Ready,
    Show,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PendingUpdateKind {
    Dartlab,
    Launcher,
}

impl PendingUpdateKind {
    fn as_str(self) -> &'static str {
        match self {
            PendingUpdateKind::Dartlab => "dartlab",
            PendingUpdateKind::Launcher => "launcher",
        }
    }
}

#[derive(Clone, Copy, Debug)]
enum UpdateDecision {
    Accept,
    Skip,
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
  .avatar-wrap {
    position: relative;
    width: 120px; height: 120px;
    margin-bottom: 24px;
    opacity: 0;
    animation: fadeIn 0.5s 0.2s forwards;
  }
  .avatar-wrap::before {
    content: '';
    position: absolute;
    inset: -24px;
    border-radius: 50%;
    background: radial-gradient(circle, rgba(234,70,71,0.35) 0%, rgba(251,146,60,0.15) 50%, transparent 70%);
    filter: blur(16px);
    animation: pulse 3s ease-in-out infinite;
  }
  .avatar {
    position: relative;
    width: 100%; height: 100%;
    border-radius: 50%;
  }
  @keyframes pulse {
    0%, 100% { opacity: 0.7; transform: scale(1); }
    50% { opacity: 1; transform: scale(1.08); }
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
  .status-row {
    display: flex;
    align-items: center;
    gap: 10px;
    margin-bottom: 4px;
    min-height: 22px;
  }
  .spinner {
    width: 14px;
    height: 14px;
    border-radius: 50%;
    border: 2px solid rgba(148,163,184,0.25);
    border-top-color: #fb923c;
    animation: spin 0.8s linear infinite;
    opacity: 0;
    transition: opacity 0.18s ease;
    pointer-events: none;
    flex: 0 0 auto;
  }
  #status {
    font-size: 13px;
    color: #64748b;
    min-height: 20px;
  }
  .maintenance-btns {
    display: flex;
    margin-top: 18px;
    gap: 8px;
    justify-content: center;
    flex-wrap: wrap;
  }
  .maintenance-btns button {
    border: 1px solid #334155;
    background: transparent;
    color: #94a3b8;
    border-radius: 999px;
    padding: 7px 14px;
    font-size: 12px;
    cursor: pointer;
    transition: background 0.2s, opacity 0.2s;
  }
  .maintenance-btns button:hover { background: rgba(148,163,184,0.08); }
  .maintenance-btns button:disabled {
    opacity: 0.45;
    cursor: not-allowed;
  }
  #error {
    color: #ea4647;
    font-size: 12px;
    margin-top: 8px;
    max-width: 480px;
    text-align: left;
    display: none;
    white-space: pre-wrap;
    user-select: all;
    overflow-y: auto;
    max-height: 180px;
    line-height: 1.5;
    padding: 0 12px;
    word-break: break-all;
  }
  .action-btns {
    display: none;
    margin-top: 12px;
    gap: 8px;
    justify-content: center;
    flex-wrap: wrap;
  }
  .action-btns button {
    border: 1px solid #ea4647;
    background: transparent;
    color: #ea4647;
    border-radius: 6px;
    padding: 8px 16px;
    font-size: 12px;
    cursor: pointer;
    font-weight: 500;
    transition: background 0.2s;
  }
  .action-btns button:hover { background: rgba(234,70,71,0.1); }
  .action-btns .btn-secondary {
    border-color: #334155;
    color: #94a3b8;
  }
  .action-btns .btn-secondary:hover { background: rgba(148,163,184,0.1); }
  #update-banner {
    display: none;
    position: fixed;
    bottom: 24px;
    background: #111827;
    border: 1px solid #1e2433;
    border-radius: 12px;
    padding: 14px 20px;
    align-items: center;
    gap: 14px;
    font-size: 13px;
    box-shadow: 0 4px 24px rgba(0,0,0,0.4);
  }
  #update-banner .ver { color: #fb923c; font-weight: 600; }
  #update-banner .label { color: #94a3b8; font-size: 11px; }
  #update-banner button {
    border: none;
    border-radius: 6px;
    padding: 6px 14px;
    font-size: 12px;
    cursor: pointer;
    font-weight: 500;
  }
  .btn-ok {
    background: #ea4647;
    color: #fff;
  }
  .btn-skip {
    background: transparent;
    color: #64748b;
  }
  @keyframes spin {
    to { transform: rotate(360deg); }
  }
  @keyframes fadeIn { to { opacity: 1; } }
</style>
</head>
<body>
  <div class="avatar-wrap"><img class="avatar" src="https://eddmpython.github.io/dartlab/avatar-analyze.png" alt=""></div>
  <h1>DartLab</h1>
  <p class="sub">AI 기업분석 준비 중</p>
  <div class="bar-wrap"><div class="bar" id="bar"></div></div>
  <div class="status-row">
    <div class="spinner" id="spinner"></div>
    <div id="status"></div>
  </div>
  <div id="error"></div>
  <div class="action-btns" id="action-btns">
    <button onclick="window.ipc.postMessage('retry')">다시 시도</button>
    <button class="btn-secondary" onclick="window.ipc.postMessage('open-log')">로그 열기</button>
    <button class="btn-secondary" onclick="window.ipc.postMessage('reset')">초기화 후 재시도</button>
  </div>
  <div class="maintenance-btns">
    <button id="ollama-toggle-btn" onclick="window.ipc.postMessage('ollama:toggle')">Ollama 설정</button>
  </div>
  <div id="update-banner">
    <div>
      <span class="label" id="update-label"></span><br>
      <span>새 버전 <span class="ver" id="update-ver"></span> 사용 가능</span>
    </div>
    <button class="btn-ok" id="update-ok-btn">업데이트</button>
    <button class="btn-skip" id="update-skip-btn">다음에</button>
  </div>
  <script>
    var promptActive = false;
    var busy = false;
    function syncUiState() {
      var disabled = busy || promptActive;
      document.getElementById('ollama-toggle-btn').disabled = disabled;
    }
    function setBusy(nextBusy) {
      busy = !!nextBusy;
      var spinner = document.getElementById('spinner');
      spinner.style.opacity = busy ? '1' : '0';
      spinner.setAttribute('aria-hidden', busy ? 'false' : 'true');
      document.body.setAttribute('data-busy', busy ? 'true' : 'false');
      syncUiState();
    }
    function setOllamaButton(label) {
      document.getElementById('ollama-toggle-btn').textContent = label;
    }
    function setStatusLabel(label) {
      document.getElementById('status').textContent = label;
    }
    function setProgress(pct, label) {
      setBusy(true);
      document.getElementById('bar').style.width = pct + '%';
      setStatusLabel(label);
    }
    function setError(msg) {
      setBusy(false);
      var el = document.getElementById('error');
      el.textContent = msg;
      el.style.display = 'block';
      document.getElementById('action-btns').style.display = 'flex';
    }
    function clearError() {
      document.getElementById('error').textContent = '';
      document.getElementById('error').style.display = 'none';
      document.getElementById('action-btns').style.display = 'none';
    }
    function showUpdate(type, ver) {
      setBusy(false);
      promptActive = true;
      syncUiState();
      var label = type === 'launcher' ? '런처' : 'DartLab';
      document.getElementById('update-label').textContent = label;
      document.getElementById('update-ver').textContent = 'v' + ver;
      document.getElementById('update-ok-btn').onclick = function() {
        window.ipc.postMessage('update:' + type + ':yes');
      };
      document.getElementById('update-skip-btn').onclick = function() {
        window.ipc.postMessage('update:' + type + ':skip');
      };
      document.getElementById('update-banner').style.display = 'flex';
    }
    function hideUpdate() {
      promptActive = false;
      syncUiState();
      document.getElementById('update-banner').style.display = 'none';
    }
    window.ipc.postMessage('ready');
  </script>
</body>
</html>"#;

const ICO_BYTES: &[u8] = include_bytes!("../assets/icon.ico");

fn main() {
    install_panic_hook();
    logger::init();
    logger::log(&format!(
        "DartLab 런처 시작 v{}",
        selfUpdate::current_version()
    ));
    selfUpdate::cleanup_old();

    if !acquire_mutex() {
        logger::log("이미 실행 중인 인스턴스 감지");
        show_already_running();
        return;
    }

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    let proxy = event_loop.create_proxy();

    let icon = load_window_icon();

    let mut wb = WindowBuilder::new()
        .with_title("DartLab")
        .with_inner_size(tao::dpi::LogicalSize::new(1200.0, 800.0))
        .with_min_inner_size(tao::dpi::LogicalSize::new(800.0, 600.0))
        .with_visible(false);

    if let Some(ic) = icon {
        wb = wb.with_window_icon(Some(ic));
    }

    let window = wb.build(&event_loop).expect("Failed to create window");

    let ipc_proxy = proxy.clone();
    let update_state: std::sync::Arc<std::sync::Mutex<UpdateState>> =
        std::sync::Arc::new(std::sync::Mutex::new(UpdateState::default()));
    let update_state_ipc = update_state.clone();

    let mut web_ctx = wry::WebContext::new(Some(paths::app_dir().join("webview2")));
    let webview = WebViewBuilder::with_web_context(&mut web_ctx)
        .with_background_color((5, 8, 17, 255))
        .with_html(SETUP_HTML)
        .with_ipc_handler(move |msg| {
            let body = msg.body();
            match body.as_str() {
                "ready" => {
                    let _ = ipc_proxy.send_event(AppEvent::Show);
                }
                "retry" => {
                    let _ = ipc_proxy.send_event(AppEvent::Log("clearError()".to_string()));
                    let p = ipc_proxy.clone();
                    let us = update_state_ipc.clone();
                    std::thread::spawn(move || {
                        let (tx, _rx) = mpsc::channel::<AppEvent>();
                        run_setup(tx, p, us);
                    });
                }
                "open-log" => {
                    let target = runner::server_log_path()
                        .filter(|p| p.exists())
                        .or_else(|| logger::log_path().filter(|p| p.exists()))
                        .or_else(logger::log_path);
                    if let Some(path) = target {
                        std::thread::spawn(move || open_in_explorer(&path));
                    }
                }
                "reset" => {
                    let _ = ipc_proxy.send_event(AppEvent::Log("clearError()".to_string()));
                    let p = ipc_proxy.clone();
                    let us = update_state_ipc.clone();
                    std::thread::spawn(move || {
                        runner::stop_server();
                        ollama::stop_ollama();
                        let ad = paths::app_dir();
                        state::clear_state(&ad);
                        let venv = paths::venv_dir(&ad);
                        if venv.exists() {
                            std::fs::remove_dir_all(&venv).ok();
                        }
                        logger::log("사용자 초기화 — venv 삭제 + 웜 스타트 상태 초기화 후 콜드 스타트");
                        let (tx, _rx) = mpsc::channel::<AppEvent>();
                        run_setup(tx, p, us);
                    });
                }
                "ollama:toggle" => {
                    let p = ipc_proxy.clone();
                    std::thread::spawn(move || {
                        let app_dir = paths::app_dir();
                        let enabled = state::ollama_enabled(&app_dir);
                        let installed = ollama::is_installed();

                        if !enabled {
                            if !confirm_native(
                                "Ollama 사용",
                                "로컬 AI용 Ollama 사용을 다시 켭니다.\n\n다음 재시도 또는 다음 실행부터 설치/기동을 진행합니다.\n\n계속할까요?",
                            ) {
                                return;
                            }

                            state::set_ollama_enabled(&app_dir, true);
                            let _ = p.send_event(AppEvent::Log(format!(
                                "clearError();setOllamaButton('{}');setStatusLabel('Ollama를 다시 사용할 수 있도록 설정했습니다. 다시 시도하면 설치를 진행합니다.')",
                                ollama_button_label(&app_dir)
                            )));
                            logger::log("사용자가 Ollama 사용을 다시 활성화");
                            return;
                        }

                        if !installed {
                            if !confirm_native(
                                "Ollama 사용 안 함",
                                "다음 재시도 또는 다음 실행부터 Ollama 설치를 건너뜁니다.\n\n계속할까요?",
                            ) {
                                return;
                            }

                            state::set_ollama_enabled(&app_dir, false);
                            let _ = p.send_event(AppEvent::Log(format!(
                                "clearError();setOllamaButton('{}');setStatusLabel('Ollama 설치를 건너뛰도록 설정했습니다. 로컬 AI 없이 계속 사용할 수 있습니다.')",
                                ollama_button_label(&app_dir)
                            )));
                            logger::log("사용자가 Ollama 설치를 비활성화");
                            return;
                        }

                        if !confirm_native(
                            "Ollama 제거",
                            "설치된 Ollama와 실행 중인 프로세스를 제거하고 로컬 AI를 끕니다.\n\n계속할까요?",
                        ) {
                            return;
                        }

                        let _ = p.send_event(AppEvent::Log("clearError()".to_string()));
                        let _ = p.send_event(AppEvent::Log(
                            "setProgress(0,'Ollama 제거 중...')".to_string(),
                        ));

                        runner::stop_server();
                        ollama::stop_ollama();

                        match ollama::uninstall_ollama() {
                            Ok(()) => {
                                state::set_ollama_enabled(&app_dir, false);
                                let _ = p.send_event(AppEvent::Log(format!(
                                    "clearError();setBusy(false);setOllamaButton('{}');setStatusLabel('Ollama 제거 완료. 이후 실행은 로컬 AI 없이 진행됩니다.')",
                                    ollama_button_label(&app_dir)
                                )));
                                logger::log("사용자가 Ollama 제거 및 비활성화 완료");
                            }
                            Err(e) => {
                                let escaped = e
                                    .replace('\\', "\\\\")
                                    .replace('\'', "\\'")
                                    .replace('\n', "\\n");
                                let _ = p.send_event(AppEvent::Log(format!(
                                    "setError('Ollama 제거 실패: {escaped}')"
                                )));
                            }
                        }
                    });
                }
                "update:dartlab:yes" => {
                    resolve_update_decision(
                        &update_state_ipc,
                        PendingUpdateKind::Dartlab,
                        UpdateDecision::Accept,
                    );
                }
                "update:launcher:yes" => {
                    resolve_update_decision(
                        &update_state_ipc,
                        PendingUpdateKind::Launcher,
                        UpdateDecision::Accept,
                    );
                }
                "update:dartlab:skip" => {
                    resolve_update_decision(
                        &update_state_ipc,
                        PendingUpdateKind::Dartlab,
                        UpdateDecision::Skip,
                    );
                }
                "update:launcher:skip" => {
                    resolve_update_decision(
                        &update_state_ipc,
                        PendingUpdateKind::Launcher,
                        UpdateDecision::Skip,
                    );
                }
                _ => {}
            }
        })
        .build(&window)
        .expect("Failed to create webview");

    let (tx, rx) = mpsc::channel::<AppEvent>();

    let setup_update_state = update_state.clone();
    std::thread::spawn(move || {
        run_setup(tx, proxy, setup_update_state);
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
                AppEvent::Show => {
                    window.set_visible(true);
                    window.set_focus();
                }
            }
        }

        match event {
            Event::UserEvent(AppEvent::Show) => {
                window.set_visible(true);
                window.set_focus();
            }
            Event::UserEvent(AppEvent::Log(ref script)) => {
                webview.evaluate_script(script).ok();
            }
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
                ollama::stop_ollama();
                *control_flow = ControlFlow::Exit;
            }
            _ => {}
        }
    });
}

struct PendingUpdatePrompt {
    kind: PendingUpdateKind,
    tx: mpsc::Sender<UpdateDecision>,
}

#[derive(Default)]
struct UpdateState {
    pending_update: Option<PendingUpdatePrompt>,
}

fn run_setup(
    tx: mpsc::Sender<AppEvent>,
    proxy: tao::event_loop::EventLoopProxy<AppEvent>,
    update_state: std::sync::Arc<std::sync::Mutex<UpdateState>>,
) {
    let js = |script: &str| {
        let _ = tx.send(AppEvent::Log(script.to_string()));
        let _ = proxy.send_event(AppEvent::Log(String::new()));
    };

    let progress = |pct: u32, label: &str| {
        let escaped = label.replace('\'', "\\'");
        js(&format!("setProgress({pct},'{escaped}')"));
        logger::log(label);
    };

    let fail = |msg: &str| {
        let mut full = msg.to_string();
        if let Some(lp) = logger::log_path() {
            if !full.contains("로그 파일:") {
                full.push_str(&format!("\n\n런처 로그: {}", lp.display()));
            }
        }
        let escaped = full
            .replace('\\', "\\\\")
            .replace('\'', "\\'")
            .replace('\n', "\\n");
        js(&format!("setError('{escaped}')"));
        logger::log(&format!("ERROR: {msg}"));
    };

    let app_dir = paths::app_dir();
    if !app_dir.exists() {
        std::fs::create_dir_all(&app_dir).ok();
    }
    sync_ollama_button(&js, &app_dir);

    progress(2, "런처 업데이트 확인 중...");
    if let Err(e) = maybe_handle_launcher_update(&js, &update_state) {
        fail(&e);
        return;
    }

    let mut warm = state::is_warm(&app_dir);

    if warm && !state::quick_health_check(&app_dir) {
        logger::log("웜 스타트 헬스체크 실패 — 콜드 스타트로 전환");
        state::clear_state(&app_dir);
        warm = false;
    }

    if warm {
        let gpu = ollama::gpu_label();
        let use_ollama;

        logger::log("DartLab 웜 스타트 검증 완료 — 최신 버전 확인");
        progress(4, "DartLab 업데이트 확인 중...");
        if let Err(e) = maybe_handle_dartlab_update(&progress, &js, &update_state, &app_dir, 4, 5) {
            fail(&e);
            return;
        }

        progress(5, &format!("Ollama 설정 확인 중... [{gpu}]"));
        match ensure_ollama_with_prompt(&app_dir, &gpu) {
            Ok(next_use_ollama) => {
                use_ollama = next_use_ollama;
                sync_ollama_button(&js, &app_dir);
            }
            Err(e) => {
                fail(&format!("Ollama 설치 실패: {e}"));
                return;
            }
        }

        if use_ollama {
            progress(15, "Ollama 시작 중...");
            if let Err(e) = ollama::ensure_serve() {
                fail(&format!("Ollama 시작 실패: {e}"));
                return;
            }
            progress(25, "LLM 모델 확인 중...");
            if let Err(e) = ollama::ensure_model() {
                fail(&format!("모델 다운로드 실패: {e}"));
                return;
            }
        } else {
            progress(25, "Ollama 없이 계속 진행 중...");
        }

        progress(40, "UI 빌드 확인 중...");
        if let Err(e) = setup::ensure_ui_build(&app_dir) {
            fail(&format!("UI 빌드 실패: {e}"));
            return;
        }

        progress(50, "서버 시작 중...");
        match runner::start_server(&app_dir, use_ollama) {
            Ok(_) => {}
            Err(e) => {
                fail(&format!("서버 시작 실패: {e}"));
                return;
            }
        }

        progress(70, "서버 응답 대기 중...");
        match runner::wait_for_server(30) {
            Ok(()) => {
                finish_startup(&app_dir, &progress, &tx, &proxy);
            }
            Err(e) => fail(&e),
        }
        return;
    }

    progress(10, "환경 준비 중...");
    if let Err(e) = setup::ensure_uv(&app_dir) {
        fail(&format!("uv 설치 실패: {e}"));
        return;
    }

    progress(25, "DartLab 설치 확인 중...");
    if let Err(e) = setup::ensure_dartlab(&app_dir) {
        fail(&format!("DartLab 설치 실패: {e}"));
        return;
    }

    logger::log("DartLab 설치 완료 후 최신 버전 확인");
    progress(40, "업데이트 확인 중...");
    if let Err(e) = maybe_handle_dartlab_update(&progress, &js, &update_state, &app_dir, 40, 45) {
        fail(&e);
        return;
    }

    let gpu = ollama::gpu_label();
    let use_ollama;
    progress(55, &format!("Ollama 설정 확인 중... [{gpu}]"));
    match ensure_ollama_with_prompt(&app_dir, &gpu) {
        Ok(next_use_ollama) => {
            use_ollama = next_use_ollama;
            sync_ollama_button(&js, &app_dir);
        }
        Err(e) => {
            fail(&format!("Ollama 설치 실패: {e}"));
            return;
        }
    }

    if use_ollama {
        progress(65, "Ollama 시작 중...");
        if let Err(e) = ollama::ensure_serve() {
            fail(&format!("Ollama 시작 실패: {e}"));
            return;
        }

        progress(70, "LLM 모델 다운로드 중...");
        if let Err(e) = ollama::ensure_model() {
            fail(&format!("모델 다운로드 실패: {e}"));
            return;
        }
    } else {
        progress(70, "Ollama 없이 계속 진행 중...");
    }

    progress(80, "UI 빌드 확인 중...");
    if let Err(e) = setup::ensure_ui_build(&app_dir) {
        fail(&format!("UI 빌드 실패: {e}"));
        return;
    }

    progress(85, "서버 시작 중...");
    match runner::start_server(&app_dir, use_ollama) {
        Ok(_) => {}
        Err(e) => {
            fail(&format!("서버 시작 실패: {e}"));
            return;
        }
    }

    progress(90, "서버 응답 대기 중...");
    match runner::wait_for_server(60) {
        Ok(()) => {
            finish_startup(&app_dir, &progress, &tx, &proxy);
        }
        Err(e) => fail(&e),
    }
}

fn finish_startup(
    app_dir: &std::path::Path,
    progress: &impl Fn(u32, &str),
    tx: &mpsc::Sender<AppEvent>,
    proxy: &tao::event_loop::EventLoopProxy<AppEvent>,
) {
    progress(100, "준비 완료!");
    state::mark_success(app_dir);
    logger::log("DartLab 실행 성공 기록 저장");

    if let Err(e) = shortcuts::ensure_shortcuts() {
        logger::log(&format!("바로가기 동기화 실패: {e}"));
    }

    std::thread::sleep(std::time::Duration::from_millis(200));
    let _ = tx.send(AppEvent::Ready);
    let _ = proxy.send_event(AppEvent::Ready);
}

fn resolve_update_decision(
    update_state: &std::sync::Arc<std::sync::Mutex<UpdateState>>,
    kind: PendingUpdateKind,
    decision: UpdateDecision,
) {
    let mut state = update_state.lock().unwrap();
    if let Some(pending) = state.pending_update.take() {
        if pending.kind == kind {
            let _ = pending.tx.send(decision);
        } else {
            state.pending_update = Some(pending);
        }
    }
}

fn prompt_for_update(
    js: &impl Fn(&str),
    update_state: &std::sync::Arc<std::sync::Mutex<UpdateState>>,
    kind: PendingUpdateKind,
    version: &str,
) -> Result<UpdateDecision, String> {
    let (decision_tx, decision_rx) = mpsc::channel();
    {
        let mut state = update_state
            .lock()
            .map_err(|_| "업데이트 상태 잠금 실패".to_string())?;
        state.pending_update = Some(PendingUpdatePrompt {
            kind,
            tx: decision_tx,
        });
    }

    let escaped = version.replace('\'', "\\'");
    js(&format!("showUpdate('{}','{escaped}')", kind.as_str()));
    logger::log(&format!("업데이트 응답 대기: {} v{version}", kind.as_str()));

    let decision = decision_rx
        .recv()
        .map_err(|_| "업데이트 응답 채널 종료".to_string())?;

    js("hideUpdate()");
    logger::log(&format!(
        "업데이트 응답 수신: {} {:?}",
        kind.as_str(),
        decision
    ));
    Ok(decision)
}

fn maybe_handle_launcher_update(
    js: &impl Fn(&str),
    update_state: &std::sync::Arc<std::sync::Mutex<UpdateState>>,
) -> Result<(), String> {
    let Some(info) = selfUpdate::check_update() else {
        return Ok(());
    };

    match prompt_for_update(js, update_state, PendingUpdateKind::Launcher, &info.version)? {
        UpdateDecision::Skip => Ok(()),
        UpdateDecision::Accept => {
            selfUpdate::apply_update(&info)?;
            selfUpdate::relaunch_updated_exe()?;
            std::process::exit(0);
        }
    }
}

fn maybe_handle_dartlab_update(
    progress: &impl Fn(u32, &str),
    js: &impl Fn(&str),
    update_state: &std::sync::Arc<std::sync::Mutex<UpdateState>>,
    app_dir: &std::path::Path,
    check_progress: u32,
    apply_progress: u32,
) -> Result<(), String> {
    logger::log("DartLab 최신 버전 확인 시작");

    match updater::check_update(app_dir) {
        Ok(Some(version)) => {
            progress(
                check_progress,
                &format!("DartLab v{version} 업데이트 확인됨"),
            );
            match prompt_for_update(js, update_state, PendingUpdateKind::Dartlab, &version)? {
                UpdateDecision::Skip => Ok(()),
                UpdateDecision::Accept => {
                    progress(
                        apply_progress,
                        &format!("DartLab v{version} 업데이트 중..."),
                    );
                    updater::do_update(app_dir)
                }
            }
        }
        Ok(None) => Ok(()),
        Err(e) => {
            logger::log(&format!("DartLab 업데이트 확인 실패: {e}"));
            Ok(())
        }
    }
}

fn ollama_button_label(app_dir: &std::path::Path) -> &'static str {
    if !state::ollama_enabled(app_dir) {
        "Ollama 사용"
    } else if ollama::is_installed() {
        "Ollama 제거"
    } else {
        "Ollama 사용 안 함"
    }
}

fn sync_ollama_button(js: &impl Fn(&str), app_dir: &std::path::Path) {
    js(&format!(
        "setOllamaButton('{}')",
        ollama_button_label(app_dir)
    ));
}

fn ensure_ollama_with_prompt(app_dir: &std::path::Path, gpu_label: &str) -> Result<bool, String> {
    if !state::ollama_enabled(app_dir) {
        logger::log("Ollama 비활성화 상태 — 설치/기동 생략");
        return Ok(false);
    }

    if !ollama::is_installed() {
        let message = format!(
            "DartLab은 로컬 AI 실행을 위해 Ollama를 설치할 수 있습니다.\n\n감지된 가속기: {gpu_label}\n예상 소요 시간: 수 분\n\n지금 설치할까요?\n\n아니오를 선택하면 Ollama 없이 계속 시작합니다."
        );
        if !confirm_native("Ollama 설치", &message) {
            state::set_ollama_enabled(app_dir, false);
            logger::log("사용자가 Ollama 설치를 건너뜀 — 로컬 AI 없이 계속 진행");
            return Ok(false);
        }
    }

    ollama::ensure_ollama(app_dir)?;
    state::set_ollama_enabled(app_dir, true);
    Ok(true)
}

fn acquire_mutex() -> bool {
    use windows_sys::Win32::Foundation::GetLastError;
    use windows_sys::Win32::System::Threading::CreateMutexW;

    const ERROR_ALREADY_EXISTS: u32 = 183;

    let name: Vec<u16> = "Global\\DartLabDesktopMutex\0".encode_utf16().collect();

    unsafe {
        let handle = CreateMutexW(std::ptr::null(), 0, name.as_ptr());

        if handle.is_null() {
            return false;
        }

        GetLastError() != ERROR_ALREADY_EXISTS
    }
}

fn show_already_running() {
    use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;

    let msg: Vec<u16> = "DartLab이 이미 실행 중입니다.\0".encode_utf16().collect();
    let title: Vec<u16> = "DartLab\0".encode_utf16().collect();

    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            msg.as_ptr(),
            title.as_ptr(),
            0x00000040,
        );
    }
}

fn confirm_native(title: &str, message: &str) -> bool {
    use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;

    const MB_YESNO: u32 = 0x00000004;
    const MB_ICONQUESTION: u32 = 0x00000020;
    const MB_DEFBUTTON2: u32 = 0x00000100;
    const IDYES: i32 = 6;

    let wide_msg: Vec<u16> = message.encode_utf16().chain(std::iter::once(0)).collect();
    let wide_title: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        MessageBoxW(
            std::ptr::null_mut(),
            wide_msg.as_ptr(),
            wide_title.as_ptr(),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        ) == IDYES
    }
}

fn install_panic_hook() {
    std::panic::set_hook(Box::new(|info| {
        let msg = format!("PANIC: {info}");
        logger::log(&msg);

        let log_path = logger::log_path()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        let display = format!(
            "DartLab에서 예기치 않은 오류가 발생했습니다.\n\n{}\n\n로그: {}",
            info, log_path
        );

        use windows_sys::Win32::UI::WindowsAndMessaging::MessageBoxW;
        let wide_msg: Vec<u16> = display.encode_utf16().chain(std::iter::once(0)).collect();
        let wide_title: Vec<u16> = "DartLab 오류\0".encode_utf16().collect();

        unsafe {
            MessageBoxW(
                std::ptr::null_mut(),
                wide_msg.as_ptr(),
                wide_title.as_ptr(),
                0x00000010,
            );
        }
    }));
}

fn load_window_icon() -> Option<Icon> {
    let ico = ICO_BYTES;

    if ico.len() < 6 {
        return None;
    }
    let count = u16::from_le_bytes([ico[4], ico[5]]) as usize;
    if count == 0 || ico.len() < 6 + count * 16 {
        return None;
    }

    let mut best_idx = 0usize;
    let mut best_area = 0u32;
    for i in 0..count {
        let p = 6 + i * 16;
        let w = if ico[p] == 0 { 256u32 } else { ico[p] as u32 };
        let h = if ico[p + 1] == 0 {
            256u32
        } else {
            ico[p + 1] as u32
        };
        if w * h > best_area {
            best_area = w * h;
            best_idx = i;
        }
    }

    let p = 6 + best_idx * 16;
    let data_size = u32::from_le_bytes([ico[p + 8], ico[p + 9], ico[p + 10], ico[p + 11]]) as usize;
    let data_offset =
        u32::from_le_bytes([ico[p + 12], ico[p + 13], ico[p + 14], ico[p + 15]]) as usize;

    if data_offset + data_size > ico.len() {
        return None;
    }
    let data = &ico[data_offset..data_offset + data_size];

    if data.len() >= 4 && data[0..4] == [0x89, 0x50, 0x4E, 0x47] {
        let decoder = png::Decoder::new(data);
        let mut reader = decoder.read_info().ok()?;
        let mut buf = vec![0u8; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).ok()?;
        buf.truncate(info.buffer_size());
        return Icon::from_rgba(buf, info.width, info.height).ok();
    }

    let entry_w = if ico[p] == 0 { 256u32 } else { ico[p] as u32 };
    let entry_h = if ico[p + 1] == 0 {
        256u32
    } else {
        ico[p + 1] as u32
    };

    if data.len() < 40 {
        return None;
    }
    let bpp = u16::from_le_bytes([data[14], data[15]]);
    if bpp != 32 {
        return None;
    }

    let pixel_count = (entry_w * entry_h) as usize;
    if data.len() < 40 + pixel_count * 4 {
        return None;
    }

    let pixels = &data[40..40 + pixel_count * 4];
    let mut rgba = vec![0u8; pixel_count * 4];
    for row in 0..entry_h as usize {
        let src_row = (entry_h as usize - 1 - row) * entry_w as usize;
        let dst_row = row * entry_w as usize;
        for col in 0..entry_w as usize {
            let si = (src_row + col) * 4;
            let di = (dst_row + col) * 4;
            rgba[di] = pixels[si + 2];
            rgba[di + 1] = pixels[si + 1];
            rgba[di + 2] = pixels[si];
            rgba[di + 3] = pixels[si + 3];
        }
    }

    Icon::from_rgba(rgba, entry_w, entry_h).ok()
}

fn open_in_explorer(path: &std::path::Path) {
    let spawn = if path.is_file() {
        Command::new("explorer")
            .arg(format!("/select,{}", path.display()))
            .spawn()
    } else if path.exists() {
        Command::new("explorer").arg(path).spawn()
    } else if let Some(parent) = path.parent() {
        Command::new("explorer").arg(parent).spawn()
    } else {
        return;
    };

    spawn.ok();
}
