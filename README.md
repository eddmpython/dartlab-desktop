<p align="center">
  <img src="https://eddmpython.github.io/dartlab/avatar-analyze.png" alt="DartLab Avatar" width="120" />
</p>

<h3 align="center">DartLab Desktop</h3>

<p align="center">
  로컬 AI 기업분석을 Windows에서 바로 실행하는 1파일 런처
</p>

<p align="center">
  <strong>DartLab.exe</strong> 하나로 Python, <code>dartlab</code>, Web UI를 준비하고, Ollama와 모델은 필요할 때만 설치한다.
</p>

<p align="center">
  <a href="https://github.com/eddmpython/dartlab-desktop/releases"><img src="https://img.shields.io/github/v/release/eddmpython/dartlab-desktop?display_name=tag" alt="Release" /></a>
  <a href="LICENSE"><img src="https://img.shields.io/github/license/eddmpython/dartlab-desktop" alt="License" /></a>
  <a href="https://github.com/eddmpython/dartlab-desktop/actions/workflows/release.yml"><img src="https://img.shields.io/github/actions/workflow/status/eddmpython/dartlab-desktop/release.yml?label=release" alt="Release Workflow" /></a>
  <a href="https://github.com/eddmpython/dartlab-desktop/releases"><img src="https://img.shields.io/badge/platform-Windows%2010%2F11-0078D6" alt="Platform" /></a>
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/runtime-Rust-000000?logo=rust" alt="Runtime" /></a>
</p>

<p align="center">
  <img src="https://img.shields.io/badge/local%20AI-optional-F97316" alt="Local AI Optional" />
  <img src="https://img.shields.io/badge/update-self--updating-111827" alt="Self Updating" />
  <img src="https://img.shields.io/badge/package-single%20exe-2563EB" alt="Single EXE" />
</p>

<p align="center">
  <a href="https://github.com/eddmpython/dartlab-desktop/releases">Releases</a> ·
  <a href="https://github.com/eddmpython/dartlab">DartLab 본체</a> ·
  <a href="https://eddmpython.github.io/dartlab/">문서</a> ·
  <a href="https://github.com/eddmpython/dartlab/blob/main/README_KR.md">한국어 문서</a> ·
  <a href="https://github.com/eddmpython/dartlab-desktop/issues">이슈</a>
</p>

`DartLab.exe` 하나만 실행하면 Python, `dartlab`, Web UI 준비까지 자동으로 끝낸다. 로컬 AI가 필요하면 Ollama 설치를 사용자에게 먼저 묻고, 원하지 않으면 건너뛴 채로 바로 시작할 수 있다. 비개발자 기준으로는 "다운로드 후 더블클릭", 개발자 기준으로는 "Windows용 self-contained launcher"가 목표다.

## 무엇을 해결하나

`dartlab` 본체는 강력하지만, 일반 사용자는 Python 환경, 패키지 설치, 로컬 LLM, 모델 다운로드, 브라우저 실행까지 직접 맞추기 어렵다. DartLab Desktop은 이 진입장벽을 없애는 Windows 네이티브 런처다.

- 첫 실행에서 필요한 런타임과 패키지를 자동 설치한다.
- 재실행 시에는 웜 스타트로 훨씬 빠르게 연다.
- Ollama와 모델은 사용자 승인 후에만 설치하고, 원하지 않으면 로컬 AI 없이 계속 사용할 수 있다.
- 런처 업데이트와 `dartlab` 본체 업데이트를 각각 분리해서 확인하고, 둘 다 사용자 승인 후에만 적용한다.
- 성공 실행 후 바탕화면과 시작 메뉴 바로가기를 자동 생성하고, 이후 누락되면 자동 복구한다.

## 빠른 시작

1. [GitHub Releases](https://github.com/eddmpython/dartlab-desktop/releases)에서 `DartLab.exe`를 다운로드한다.
2. `DartLab.exe`를 실행한다.
3. 첫 실행에서는 자동 설치가 진행된다. 네트워크와 디스크 상태에 따라 몇 분 정도 걸릴 수 있다.
4. 로컬 AI를 쓰고 싶다면 Ollama 설치 확인 창에서 `예`를 누른다. `아니오`를 누르면 Ollama 없이 계속 시작한다.
5. 준비가 끝나면 `http://127.0.0.1:8400` 기반 UI로 자동 전환된다.

## 첫 실행에서 자동 구성되는 것

| 구성 요소 | 역할 |
|---|---|
| [uv](https://docs.astral.sh/uv/) | Python 환경과 패키지 설치 |
| Python 3.12 | `uv venv`로 격리 환경 생성 |
| [`dartlab[ai,llm]`](https://github.com/eddmpython/dartlab) | 분석 본체와 AI 기능 |
| [Ollama](https://ollama.com) | 로컬 LLM 런타임, 사용자 승인 시에만 설치 |
| `qwen3:4b` | 기본 한국어 분석 모델, Ollama 사용 시에만 다운로드 |
| WebView2 데이터 | 런처 UI와 내장 브라우저 상태 |

모든 데이터는 `%LOCALAPPDATA%\DartLab\` 아래에 격리된다. 시스템 Python이나 전역 패키지에 의존하지 않는다.

## 업데이트 방식

### 런처 업데이트

- 실행 초기에 GitHub Releases 최신 버전을 확인한다.
- 새 버전이 있으면 사용자 응답을 받을 때까지 시작을 멈춘다.
- `업데이트`를 누르면 새 `DartLab.exe`를 받아 교체하고, 새 exe를 다시 실행한 뒤 현재 프로세스는 종료한다.
- 이전 업데이트의 `.exe.old` 또는 `.exe.new` 잔여 파일은 시작 시 정리한다.

### DartLab 본체 업데이트

- `updater::check_update()`가 `dartlab` 최신 확인의 단일 기준이다.
- cold start에서는 `dartlab` 설치/검증 직후, warm start에서는 실행 준비 검증 직후 같은 최신 확인 게이트를 지난다.
- 로컬 버전과 PyPI 최신 버전을 비교하고, 새 버전이 있으면 사용자 응답을 받을 때까지 다음 단계로 넘어가지 않는다.
- 로그에는 `설치/검증 완료 -> 로컬 버전 -> PyPI 최신 버전 -> 업데이트 필요 여부`가 남는다.

## Ollama 사용 방식

- Ollama는 필수가 아니다. 설치 전 확인 창이 먼저 뜬다.
- `아니오`를 누르면 `dartlab` 서버는 Ollama 환경 변수 없이 시작되고, 앱은 로컬 AI 없이 계속 열린다.
- 런처 하단의 Ollama 버튼으로 "이번부터 설치 건너뛰기", "다시 사용", "설치된 Ollama 제거"를 처리할 수 있다.

## 바로가기

- 첫 성공 실행 후 아래 두 위치에 `DartLab.lnk`를 생성한다.
- 이후에도 성공 실행마다 누락된 바로가기를 자동 복구한다.
- 바로가기는 per-user 범위만 다룬다. 시스템 전체 설치나 인스톨러 등록은 아직 포함하지 않는다.

| 위치 | 경로 |
|---|---|
| Desktop | `%USERPROFILE%\Desktop\DartLab.lnk` |
| Start Menu | `%APPDATA%\Microsoft\Windows\Start Menu\Programs\DartLab.lnk` |

## 동작 개요

```text
DartLab.exe
├─ logger        런처 로그 파일 생성/정리
├─ selfUpdate    런처 최신 버전 확인 및 exe 교체
├─ setup         uv / Python / dartlab 설치
├─ updater       PyPI 최신 버전 비교 및 본체 업데이트
├─ ollama        Ollama 설치, 구동, 모델 확인
├─ runner        dartlab ai 서버 실행 및 대기
├─ shortcuts     Desktop / Start Menu 바로가기 생성
├─ state         웜 스타트 상태 기록
└─ wry/tao       설치 UI + WebView2 창 관리
```

상세 기술 설명은 [TECH_SPEC.md](TECH_SPEC.md)에서 확인할 수 있다.

## 시스템 요구사항

| 항목 | 최소 | 권장 |
|---|---|---|
| OS | Windows 10 | Windows 11 |
| RAM | 8GB | 16GB |
| 디스크 | 5GB 여유 | 10GB 이상 |
| GPU | 없음, CPU 모드 가능 | NVIDIA GPU |
| WebView2 | Windows 기본 내장 또는 설치 필요 | 기본 내장 |

## 트러블슈팅

### 로그 위치

```text
%LOCALAPPDATA%\DartLab\logs\
```

- 런처 로그와 서버 로그를 남긴다.
- 에러 화면의 `로그 열기` 버튼으로 바로 탐색기를 열 수 있다.

### 포트 충돌

- 기본 포트는 `8400`이다.
- 이미 다른 프로세스가 사용 중이면 실행을 막고 명확한 에러를 보여준다.
- 중복 실행은 Named Mutex로 차단한다.

### 완전 초기화

- 에러 화면의 `초기화 후 재시도` 버튼을 사용할 수 있다.
- 수동으로는 `%LOCALAPPDATA%\DartLab\`를 삭제하면 다음 실행에서 처음부터 다시 설치한다.

### HTTPS 다운로드 인증서 문제

- 일부 기업/프록시 환경에서는 Rust HTTP TLS 검증이 실패할 수 있다.
- 이런 경우를 대비해 HTTPS 요청/다운로드는 PowerShell fallback을 사용해 Windows 인증서 저장소 경로로 한 번 더 시도한다.

## 개발

```bash
cargo build --release
```

로컬 실행:

```bash
cargo run --release
```

## 배포

태그를 푸시하면 GitHub Actions가 Windows 릴리즈 빌드를 만들고 `DartLab.exe`를 Release에 업로드한다.

```bash
git tag vX.Y.Z
git push origin main vX.Y.Z
```

현재 워크플로우:

- `cargo build --release`
- `dartlab-desktop.exe`를 `DartLab.exe`로 리네이밍
- build provenance attestation 생성
- GitHub Release 업로드

## 관련 프로젝트

| 프로젝트 | 설명 |
|---|---|
| [dartlab](https://github.com/eddmpython/dartlab) | Python 본체 |
| [dartlab landing](https://github.com/eddmpython/dartlab/tree/main/landing) | 랜딩 페이지 |

## 라이선스

MIT
