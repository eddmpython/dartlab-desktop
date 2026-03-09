# DartLab Desktop

DartLab AI 기업분석을 **더블클릭 한 번**으로 실행하는 Windows 런처.

Python, uv, Ollama를 모를 필요 없다. exe를 실행하면 전부 자동으로 설치되고 브라우저에서 AI 기업분석이 열린다.

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

## 업데이트

실행할 때마다 PyPI에서 최신 버전을 확인한다. 새 버전이 있으면 자동으로 업데이트.

## 빌드

```bash
cargo build --release
```

## 라이선스

MIT
