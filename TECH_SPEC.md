# DartLab Desktop — 기술 스펙

## 개요

비개발자가 더블클릭 한 번으로 DartLab AI 기업분석을 사용할 수 있는 Windows 네이티브 앱.
Rust 바이너리가 Python 환경 + LLM 엔진을 자동 관리하고, WebView2로 UI를 표시한다.

## 아키텍처

```
DartLab.exe (Rust, ~2.7MB)
├── main          → WebView2 윈도우 + IPC + 이벤트 루프 + 뮤텍스 + panic hook
├── selfUpdate    → GitHub Releases API로 exe 자체 업데이트 (사용자 승인)
├── setup         → uv 다운로드 + Python venv 생성 + dartlab[ai,llm] 설치
├── updater       → PyPI 버전 체크 + dartlab 패키지 업데이트 (사용자 승인)
├── ollama        → Ollama 설치 + GPU 감지 + 모델 다운로드 + 프리로드
├── runner        → dartlab ai 서버 subprocess 관리
├── state         → 웜/콜드 스타트 판별 (state.json)
├── logger        → 파일 기반 로깅 + 자동 정리
└── paths         → 경로 관리 (%LOCALAPPDATA%\DartLab)
```

## 실행 흐름

```
더블클릭
  │
  ├─ 0. Panic hook 설치 + 로거 초기화
  ├─ 1. Named Mutex 획득 (중복 실행 방지)
  │     └─ 이미 실행 중 → MessageBox 알림 후 종료
  ├─ 2. 이전 .old 파일 정리 (자동 업데이트 잔여물)
  │
  ├─ 3. WebView2 윈도우 생성 (hidden) + IPC 핸들러 등록
  │     └─ HTML ready → set_visible(true) (흰색 깜빡임 방지)
  │
  ├─ [백그라운드: 런처 자체 업데이트 체크]
  │   └─ 새 버전 발견 → IPC 배너로 사용자 승인 요청
  │
  ├─ [웜 스타트] (state.json 7일 이내)
  │   ├─ GPU 레이블 표시
  │   ├─ 백그라운드: dartlab 업데이트 체크 → IPC 배너
  │   ├─ 서버 시작 + 모델 프리로드
  │   └─ 서버 응답 대기 → WebView URL 전환
  │
  └─ [콜드 스타트]
      ├─ 4. uv 설치 확인/다운로드
      ├─ 5. Python venv + dartlab[ai,llm] 설치
      ├─ 6. PyPI 업데이트 체크 + 자동 적용
      ├─ 7. GPU 감지 + Ollama 설치 + 모델 확인
      ├─ 8. dartlab ai 서버 시작 + 모델 프리로드
      └─ 9. TCP 응답 대기 → WebView URL 전환
```

## 기술 스택

| 항목 | 기술 | 용도 |
|------|------|------|
| 언어 | Rust (edition 2024) | 런처 + 윈도우 관리 |
| 윈도우 | tao 0.33 | 네이티브 윈도우 생성 |
| 웹뷰 | wry 0.49 (WebView2) | 설치 UI + dartlab UI 표시 |
| HTTP | ureq 3 (동기) | 다운로드, API 호출 (tokio 불필요) |
| JSON | serde_json | PyPI/GitHub API 파싱 |
| 경로 | dirs | %LOCALAPPDATA% 접근 |
| ZIP | zip (native) | uv ZIP 추출 (PowerShell 불필요) |
| 아이콘 | png 0.17 | ICO 내 PNG 디코딩 |
| Win32 | windows-sys 0.59 | Named Mutex, MessageBox |
| 빌드 | winresource | exe 아이콘 임베딩 |

## 파일 구조

```
src/
├── main.rs         → 진입점, Mutex, panic hook, WebView IPC, 이벤트 루프
├── setup.rs        → uv 다운로드 + Python venv + dartlab 설치
├── updater.rs      → PyPI 버전 비교 + 패키지 업데이트
├── runner.rs       → dartlab ai subprocess 관리
├── ollama.rs       → Ollama 설치 + GPU 감지 + 모델 프리로드
├── paths.rs        → 경로 유틸리티
├── selfUpdate.rs   → exe 자체 업데이트 (사용자 승인 기반)
├── state.rs        → 웜/콜드 스타트 판별 (state.json)
└── logger.rs       → 파일 로깅 + 7일 자동 정리
```

## 데이터 디렉토리

```
%LOCALAPPDATA%\DartLab\
├── uv/
│   └── uv.exe           → astral-sh/uv 바이너리
├── .venv/
│   └── Scripts/
│       ├── python.exe   → Python 3.12
│       └── dartlab.exe  → dartlab CLI
├── webview2/            → WebView2 데이터 격리
├── logs/
│   └── dartlab-{ts}.log → 실행 로그 (7일 자동 삭제)
└── state.json           → 웜 스타트 상태 기록
```

## 제품급 기능

### 중복 실행 방지 (Named Mutex)
- `Global\DartLabDesktopMutex` Win32 Named Mutex
- 이미 실행 중이면 MessageBox 알림 후 즉시 종료
- 포트 충돌(8400) 문제 원천 차단

### 사용자 승인 기반 업데이트
- **dartlab 패키지**: 웜 스타트 시 백그라운드 체크 → IPC 배너로 승인 요청
- **런처 자체**: 백그라운드 GitHub Releases 체크 → IPC 배너로 승인 요청
- 배너 UI: "런처/DartLab" 라벨 + 버전 + 업데이트/다음에 버튼
- 사용자가 "다음에" 선택 시 무시, "업데이트" 시 백그라운드 적용

### 에러 재시도
- 설치 실패 시 에러 메시지 + "다시 시도" 버튼 표시
- IPC를 통해 setup 스레드를 재실행
- clearError() → 에러 UI 초기화 후 처음부터 재시도

### 로그 파일
- `%LOCALAPPDATA%\DartLab\logs\dartlab-{unix_ts}.log`
- 모든 단계 진행 상황 + 에러 기록
- 7일 이상 된 로그 자동 삭제 (시작 시 정리)
- panic 발생 시 로그 경로를 MessageBox에 표시

### 크래시 리포팅 (Panic Hook)
- `std::panic::set_hook`으로 전역 panic 핸들러 설치
- panic 정보를 로그 파일에 기록
- Windows MessageBox로 사용자에게 오류 + 로그 경로 안내

### 흰색 깜빡임 방지
- `with_visible(false)`로 윈도우 숨김 생성
- `with_background_color((5, 8, 17, 255))`로 WebView 배경 설정
- HTML 로드 완료 시 IPC `ready` → `set_visible(true)`

### 웜 스타트 (state.json)
- 성공적 실행 완료 시 `{"last_success": unix_ts}` 기록
- 7일 이내 성공 기록 → uv/dartlab/ollama 설치 체크 스킵
- 서버 바로 시작 → 체감 시작 시간 대폭 단축

### GPU 감지 + 모델 프리로드
- `wmic path win32_VideoController` → GPU 이름/벤더 파싱
- NVIDIA → "CUDA", AMD → "ROCm", Intel → "GPU", 없음 → "CPU"
- 설치 UI에 GPU 레이블 표시
- 서버 시작 직후 `POST /api/generate` (keep_alive:-1)로 모델 메모리 로드
- 첫 질문 시 콜드 스타트 지연 제거

## 자동 업데이트 메커니즘

### exe 자체 (selfUpdate)
1. GitHub API `GET /repos/eddmpython/dartlab-desktop/releases/latest`
2. `tag_name`과 현재 `CARGO_PKG_VERSION` 비교
3. 새 버전이면 IPC 배너로 사용자 승인 요청
4. 승인 시: `DartLab.exe` 에셋 다운로드 → `.exe.new`
5. 현재 exe → `.exe.old` 리네이밍
6. `.exe.new` → 원래 이름 리네이밍
7. 다음 실행 시 `.old` 자동 삭제

### Python 패키지 (updater)
1. PyPI API `GET /pypi/dartlab/json` → 최신 버전
2. venv 내 `python -c "import dartlab; print(dartlab.__version__)"` → 현재 버전
3. 다르면 IPC 배너 표시 (웜 스타트) 또는 자동 적용 (콜드 스타트)
4. `uv pip install --upgrade dartlab[ai,llm]`

## IPC 프로토콜

WebView ↔ Rust 간 통신:

| 메시지 | 방향 | 동작 |
|--------|------|------|
| `ready` | JS → Rust | 윈도우 표시 |
| `retry` | JS → Rust | 에러 초기화 + setup 재실행 |
| `update:dartlab:yes` | JS → Rust | dartlab 패키지 업데이트 실행 |
| `update:dartlab:skip` | JS → Rust | 배너 숨김 |
| `update:launcher:yes` | JS → Rust | 런처 exe 업데이트 실행 |
| `update:launcher:skip` | JS → Rust | 배너 숨김 |
| `setProgress(pct, label)` | Rust → JS | 진행률 업데이트 |
| `setError(msg)` | Rust → JS | 에러 표시 + 재시도 버튼 |
| `clearError()` | Rust → JS | 에러 UI 초기화 |
| `showUpdate(type, ver)` | Rust → JS | 업데이트 배너 표시 |
| `hideUpdate()` | Rust → JS | 업데이트 배너 숨김 |

## 빌드 & 배포

```bash
cargo build --release        # 로컬 빌드 (~2.7MB)
cargo run --release           # 로컬 테스트
```

GitHub Actions (tag push → 자동):
1. `git tag vX.Y.Z && git push origin vX.Y.Z`
2. `windows-latest`에서 `cargo build --release` (rust-cache 적용)
3. `dartlab-desktop.exe` → `DartLab.exe` 리네이밍
4. GitHub Release 생성 + 에셋 업로드

## 해결한 문제들

### Stdio::from(file)이 uvicorn을 블로킹 (v0.3.2)

**증상** — 서버 프로세스가 spawn되지만 응답하지 않음. 30초 후 타임아웃.

**원인** — `Stdio::from(file)` 형태로 stdout을 리다이렉트했는데, Windows에서 파일 I/O가 uvicorn의 async event loop를 블로킹함.

**해결** — `Stdio::null()`로 변경.

### localhost가 IPv6로 해석됨 (v0.3.2)

**증상** — TCP connect로 서버 대기 시 연결 실패.

**원인** — Windows가 `localhost`를 `::1` (IPv6)로 먼저 해석.

**해결** — 모든 URL/연결에서 `localhost` → `127.0.0.1`로 변경.

### 서버 health check가 너무 느림 (v0.3.2)

**원인** — `/api/status`가 `detect_ollama()` + `provider.check_available()`을 동기로 실행.

**해결** — HTTP health check → `TcpStream::connect_timeout`으로 변경.

### 포트 이미 사용 중일 때 에러 (v0.3.2)

**해결** — `is_port_in_use()` 체크 + Named Mutex로 중복 실행 원천 방지.

### 구버전 파일 잔여 (v0.3.2)

**해결** — `cleanup_legacy()` 함수가 `pyproject.toml`, `uv.lock`, `venv/` 자동 삭제.

### WebView 리사이즈 안 됨 (v0.3.2)

**해결** — `WindowEvent::Resized` 이벤트에서 `webview.set_bounds()` 호출.

### openai 패키지 누락 (v0.3.2)

**해결** — `dartlab[ai]` → `dartlab[ai,llm]`으로 변경.

### 창이 다른 창 뒤에 숨겨짐 (v0.3.3)

**해결** — `window.set_focus()` 추가.

### 타이틀바 아이콘이 기본 아이콘 (v0.3.3)

**원인** — ICO 파일이 PNG 엔트리만 포함. 기존 image crate는 PNG ICO 미지원.

**해결** — `png` crate로 ICO 내 PNG 디코딩 → `Icon::from_rgba()`.

### 흰색 깜빡임 (v0.3.4)

**원인** — 윈도우 표시 후 WebView 렌더링까지 흰색 배경 노출.

**해결** — `with_visible(false)` → IPC `ready` → `set_visible(true)`.

## 성능 최적화 이력

| 변경 | 효과 |
|------|------|
| reqwest+tokio → ureq (동기) | 바이너리 크기 감소, 빌드 시간 단축 |
| image crate → png crate | 의존성 경량화 |
| PowerShell Expand-Archive → zip crate | 외부 프로세스 제거 |
| selfUpdate 백그라운드화 | 창 표시 차단 제거 |
| state.json 웜 스타트 | 재실행 시 설치 체크 스킵 |
| 모델 프리로드 | LLM 첫 응답 지연 제거 |

## 의존성 최소화 원칙

- Rust 크레이트 8개 (ureq, serde_json, dirs, tao, wry, zip, png, windows-sys)
- tokio/async 런타임 없음 (동기 전용)
- Python 환경은 사용자 시스템에 설치하지 않음 (%LOCALAPPDATA% 격리)
- WebView2 데이터도 앱 디렉토리에 격리
- 시스템 요구사항: Windows 10/11 (WebView2 기본 내장)

## 향후 로드맵

### 단기 (v0.4.x)
- [ ] Windows 토스트 알림 (업데이트 가용 시 시스템 알림)
- [ ] 시스템 트레이 아이콘 (최소화 시 트레이로)
- [ ] 시작 프로그램 등록 옵션

### 중기 (v0.5.x)
- [ ] llama.cpp 직접 실행 모드 (Ollama 대체, ~5MB exe)
- [ ] 모델 선택 UI (빠름 1.7B / 균형 4B / 정확 8B)
- [ ] Ollama 모델 관리 UI 통합

### 장기 (v1.0)
- [ ] Windows 인스톨러 (NSIS/WiX) — 시작메뉴, 바탕화면 바로가기
- [ ] EV 코드 서명 — SmartScreen 경고 제거
- [ ] macOS 지원 (WebKit) — Universal Binary
- [ ] 오프라인 모드 — 번들된 Python + dartlab wheel
