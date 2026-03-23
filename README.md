# DartLab Desktop

DartLab AI 기업분석을 **더블클릭 한 번**으로 실행하는 Windows 네이티브 런처.

Python, 패키지 매니저, LLM 엔진을 몰라도 된다. `DartLab.exe`를 실행하면 필요한 환경이 자동으로 구성되고, 브라우저에서 AI 기업분석이 바로 열린다. Rust 단일 바이너리 ~2.7MB.

## 주요 기능

| 기능 | 설명 |
|------|------|
| **원클릭 실행** | 더블클릭만으로 Python + LLM + 서버 자동 설치 및 실행 |
| **로컬 AI** | Ollama 기반 로컬 LLM — 데이터가 외부로 나가지 않음 |
| **GPU 자동 감지** | NVIDIA(CUDA), AMD(ROCm), Intel(GPU), CPU 자동 판별 |
| **웜 스타트** | 재실행 시 5초 내 브라우저 오픈 |
| **자동 업데이트** | 런처 + DartLab 패키지 모두 사용자 승인 기반 업데이트 |
| **크래시 복구** | 에러 발생 시 로그 경로 안내 + 재시도 버튼 |

## 다운로드

[GitHub Releases](https://github.com/eddmpython/dartlab-desktop/releases)에서 `DartLab.exe`를 다운로드.

## 사용법

1. `DartLab.exe` 더블클릭
2. 첫 실행: 환경 자동 설치 (2~5분)
3. 이후 실행: 5초 내 브라우저 오픈

## 자동 설치 항목

| 항목 | 설명 |
|------|------|
| [uv](https://docs.astral.sh/uv/) | Python 패키지 매니저 |
| Python 3.12+ | uv가 자동 설치 |
| [DartLab](https://github.com/eddmpython/dartlab) | AI 기업분석 라이브러리 |
| [Ollama](https://ollama.com) | 로컬 LLM 실행 엔진 |
| qwen3 | 한국어 재무분석 추천 모델 |

모든 항목은 `%LOCALAPPDATA%\DartLab\`에 격리 설치된다. 시스템 환경을 오염시키지 않는다.

## 시스템 요구사항

| 항목 | 최소 | 권장 |
|------|------|------|
| OS | Windows 10 | Windows 11 |
| RAM | 8GB | 16GB |
| 디스크 | 5GB | 10GB |
| GPU | 없음 (CPU 모드) | NVIDIA GPU (CUDA) |
| 런타임 | WebView2 (Windows 10/11 기본 내장) | — |

## 업데이트

실행할 때마다 백그라운드에서 새 버전을 확인한다.

- **DartLab 패키지** — PyPI에서 최신 버전 체크. 새 버전이 있으면 배너로 알림.
- **런처 자체** — GitHub Releases에서 최신 exe 체크. 새 버전이 있으면 배너로 알림.

두 경우 모두 사용자가 "업데이트" 버튼을 눌러야 적용된다. 강제 업데이트 없음.

## 아키텍처

```
DartLab.exe (Rust, ~2.7MB)
├── main          WebView2 윈도우 + IPC + 이벤트 루프
├── setup         uv + Python + dartlab 자동 설치
├── runner        dartlab ai 서버 subprocess 관리
├── updater       PyPI 버전 체크 + 패키지 업데이트
├── selfUpdate    런처 exe 자체 업데이트
├── ollama        GPU 감지 + Ollama 설치 + 모델 관리
├── state         웜/콜드 스타트 판별
├── logger        파일 로깅 + 7일 자동 정리
└── paths         경로 관리
```

상세 기술 스펙은 [TECH_SPEC.md](TECH_SPEC.md) 참조.

## 기술 스택

| 항목 | 기술 |
|------|------|
| 언어 | Rust (edition 2024) |
| 윈도우 | tao + wry (WebView2) |
| HTTP | ureq (동기, tokio 불필요) |
| ZIP | zip crate (네이티브) |
| Win32 | windows-sys (Named Mutex) |

의존성 8개. async 런타임 없음. 바이너리 크기 최적화 (strip + LTO).

## 트러블슈팅

**로그 확인**

```
%LOCALAPPDATA%\DartLab\logs\
```

7일간 보관, 이후 자동 삭제.

**포트 충돌 (8400)**

DartLab이 이미 실행 중이면 Named Mutex가 중복 실행을 차단한다. 다른 프로세스가 포트를 점유 중이면 해당 프로세스를 종료 후 재실행.

**완전 재설치**

`%LOCALAPPDATA%\DartLab\` 폴더를 삭제하면 다음 실행 시 처음부터 다시 설치한다.

## 빌드

```bash
cargo build --release
```

**배포** — tag push 시 GitHub Actions가 자동 빌드 + Releases 업로드:

```bash
git tag vX.Y.Z && git push origin vX.Y.Z
```

## 관련 프로젝트

| 프로젝트 | 설명 |
|---------|------|
| [dartlab](https://github.com/eddmpython/dartlab) | DartLab 본체 (Python, PyPI) |
| [dartlab landing](https://github.com/eddmpython/dartlab/tree/main/landing) | 랜딩 페이지 (SvelteKit, GitHub Pages) |

## 라이선스

MIT
