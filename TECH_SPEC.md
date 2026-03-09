# DartLab Desktop — 기술 스펙

## 개요

비개발자가 더블클릭 한 번으로 DartLab AI 기업분석을 사용할 수 있는 Windows 네이티브 앱.
Rust 바이너리가 Python 환경 + LLM 엔진을 자동 관리하고, WebView2로 UI를 표시한다.

## 아키텍처

```
DartLab.exe (Rust, ~2.4MB)
├── selfUpdate    → GitHub Releases API로 exe 자체 자동 업데이트
├── setup         → uv 다운로드 + Python venv 생성 + dartlab[ai] 설치
├── updater       → PyPI 버전 체크 + dartlab 패키지 업데이트
├── ollama        → Ollama 설치 + 기본 모델(qwen3) 다운로드
├── runner        → dartlab ai 서버를 subprocess로 실행
└── WebView2      → 설치 진행 화면 → localhost:8400 전환
```

## 실행 흐름

```
더블클릭
  │
  ├─ 1. 이전 .old 파일 정리 (자동 업데이트 잔여물)
  ├─ 2. GitHub Releases로 exe 자체 업데이트 체크
  │     └─ 새 버전 → 다운로드 + 교체 (다음 실행 시 적용)
  │
  ├─ 3. WebView2 윈도우 열림 (설치 진행 화면)
  │
  ├─ [백그라운드 스레드]
  │   ├─ 4. uv 설치 확인/다운로드
  │   ├─ 5. Python venv + dartlab[ai] 설치
  │   ├─ 6. PyPI 업데이트 체크 + 적용
  │   ├─ 7. Ollama 설치 + 모델 확인
  │   ├─ 8. dartlab ai 서버 시작 (subprocess)
  │   └─ 9. /api/status 응답 대기
  │
  └─ 10. WebView URL을 localhost:8400으로 전환
         (Svelte UI가 그대로 표시됨)
```

## 기술 스택

| 항목 | 기술 | 용도 |
|------|------|------|
| 언어 | Rust | 런처 + 윈도우 관리 |
| 윈도우 | tao | 네이티브 윈도우 생성 (Tauri 팀) |
| 웹뷰 | wry (WebView2) | Svelte UI 표시 (Tauri 팀) |
| HTTP | reqwest (blocking) | 다운로드, API 호출 |
| JSON | serde_json | PyPI/GitHub API 파싱 |
| 경로 | dirs | %LOCALAPPDATA% 접근 |
| 아이콘 | winresource | exe 아이콘 임베딩 |

## 파일 구조

```
src/
├── main.rs         → 진입점, WebView 윈도우 + 이벤트 루프
├── setup.rs        → uv 다운로드 + Python venv + dartlab 설치
├── updater.rs      → PyPI 버전 비교 + 패키지 업데이트
├── runner.rs       → dartlab ai subprocess 관리
├── ollama.rs       → Ollama 설치 + 모델 관리
├── paths.rs        → 경로 유틸리티
├── selfUpdate.rs   → exe 자체 자동 업데이트
└── ui.rs           → 레거시 콘솔 출력 (no-op 스텁)
```

## 데이터 디렉토리

```
%LOCALAPPDATA%\DartLab\
├── uv/
│   └── uv.exe          → astral-sh/uv 바이너리
├── .venv/
│   └── Scripts/
│       ├── python.exe  → Python 3.12
│       └── dartlab.exe → dartlab CLI
└── (dartlab이 생성하는 데이터 캐시)
```

## 자동 업데이트 메커니즘

### exe 자체 (selfUpdate)
1. GitHub API `GET /repos/eddmpython/dartlab-desktop/releases/latest`
2. `tag_name`과 현재 `CARGO_PKG_VERSION` 비교
3. 새 버전이면 `DartLab.exe` 에셋 다운로드 → `.exe.new`
4. 현재 exe → `.exe.old` 리네이밍
5. `.exe.new` → 원래 이름 리네이밍
6. 다음 실행 시 `.old` 자동 삭제

### Python 패키지 (updater)
1. PyPI API `GET /pypi/dartlab/json` → 최신 버전
2. venv 내 `python -c "import dartlab; print(dartlab.__version__)"` → 현재 버전
3. 다르면 `uv pip install --upgrade dartlab[ai]`

## 빌드 & 배포

```bash
cargo build --release        # 로컬 빌드
```

GitHub Actions (tag push → 자동):
1. `windows-latest`에서 `cargo build --release`
2. `dartlab-desktop.exe` → `DartLab.exe` 리네이밍
3. GitHub Release 생성 + 에셋 업로드

## 향후 로드맵

### 단기 (v0.3.x)
- [ ] 설치 진행 화면 디자인 개선 (DartLab 브랜드 색상, 아바타)
- [ ] 에러 발생 시 "재시도" 버튼
- [ ] 윈도우 타이틀에 연결된 기업명 표시

### 중기 (v0.4.x)
- [ ] 시스템 트레이 아이콘 (최소화 시 트레이로)
- [ ] 시작 프로그램 등록 옵션
- [ ] Ollama 모델 관리 (다운로드/삭제) UI 통합

### 장기 (v1.0)
- [ ] Windows 인스톨러 (NSIS/WiX) — 시작메뉴 등록, 바탕화면 바로가기
- [ ] EV 코드 서명 — SmartScreen 경고 제거
- [ ] macOS 지원 (WebKit) — Universal Binary
- [ ] 오프라인 모드 — 번들된 Python + dartlab wheel
- [ ] Tauri v2 마이그레이션 검토 (IPC, 플러그인 생태계)

## 의존성 최소화 원칙

- Rust 크레이트는 최소한으로 유지 (현재 5개)
- Python 환경은 사용자 시스템에 설치하지 않음 (%LOCALAPPDATA% 격리)
- 시스템 요구사항: Windows 10/11 (WebView2 기본 내장)
