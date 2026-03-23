# 빌드 가이드 (Build Guide)

## 1. 사전 요구사항

### 1.1. 필수 도구
| 도구 | 최소 버전 | 용도 |
|------|-----------|------|
| Rust (rustc + cargo) | 1.85.0+ (Edition 2024 지원) | 컴파일러 및 패키지 매니저 |
| Git | 2.40+ | 버전 관리 |

### 1.2. 플랫폼별 추가 요구사항

#### Windows
- MSVC 빌드 도구 (Visual Studio Build Tools 2022 이상)
- `x86_64-pc-windows-msvc` 타겟 (기본 설치됨)

#### Linux (x86_64)
- GCC 또는 Clang
- `pkg-config`, `libssl-dev` (배포판에 따라 이름 상이)
- `x86_64-unknown-linux-gnu` 타겟

#### Raspberry Pi (aarch64) — 네이티브 빌드 권장
`rusqlite`와 `zstd`가 C 소스를 번들 컴파일하므로, **Pi에서 직접 빌드**가 가장 안정적입니다.

```bash
# 1. Rust 설치 (최초 1회, ~2분)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# 2. 빌드 도구 설치 (Debian/Raspbian)
sudo apt update && sudo apt install -y build-essential pkg-config

# 3. 클론 + 릴리즈 빌드 (~3-5분)
git clone https://github.com/Yupkidangju/femtoClaw.git
cd femtoClaw
cargo build --release

# 4. 실행
./target/release/femtoclaw
```

> **참고:** RPi 4 (4GB+) 이상 권장. RPi 3은 RAM 부족으로 빌드 시간이 길어질 수 있습니다.

## 2. 의존성 크레이트

`spec.md`에 명시된 핵심 의존성 목록입니다.

| 크레이트 | 버전 | 역할 |
|----------|------|------|
| `ratatui` | 0.30.0 | TUI 프레임워크 |
| `crossterm` | - | 터미널 백엔드 (ratatui 연동) |
| `rusqlite` | 0.39.0 | SQLite DB (WAL 모드) |
| `zstd` | - | 트랜잭션 로그 ZSTD 압축 |
| `reqwest` | - | LLM API / 텔레그램 HTTP 클라이언트 |
| `teloxide` | - | 텔레그램 봇 프레임워크 |
| `chacha20poly1305` | - | 양방향 암복호화 (RustCrypto) |
| `dirs` | - | 크로스 플랫폼 홈 디렉토리 탐색 |

## 3. 빌드 절차

### 3.1. 소스 코드 클론
```bash
git clone https://github.com/<owner>/femtoClaw.git
cd femtoClaw
```

### 3.2. 디버그 빌드 (개발용)
```bash
cargo build
```

### 3.3. 릴리스 빌드 (프로덕션용)
```bash
cargo build --release
```
- 출력 위치: `target/release/femtoclaw` (Linux) 또는 `target/release/femtoclaw.exe` (Windows)

### 3.4. 크로스 컴파일 (Raspberry Pi)
```bash
# 타겟 추가 (최초 1회)
rustup target add aarch64-unknown-linux-gnu

# 빌드
cargo build --release --target aarch64-unknown-linux-gnu
```

## 4. 실행 방법

### 4.1. TUI 모드 (기본)
```bash
./target/release/femtoclaw
```
- 최초 실행 시 마스터 비밀번호 설정 → 온보딩 화면 진입
- `~/.femtoclaw/` 디렉토리가 자동 생성됨

### 4.2. 헤드리스(백그라운드) 모드
```bash
./target/release/femtoclaw --headless
```

## 5. 테스트 실행
```bash
# 전체 테스트
cargo test

# 특정 모듈 테스트
cargo test --lib security
cargo test --lib db
```

## 6. 코드 품질 검사
```bash
# Clippy 린트 (CI와 동일한 기준)
cargo clippy --all-targets -- -W clippy::all

# 포맷 검사
cargo fmt --check
```

> **참고:** `-W clippy::all`은 경고로 보고합니다. 안정화 후 `-D warnings`로 승격 예정.

## 7. 트러블슈팅

### SQLite 링킹 오류 (Linux)
```bash
# Debian/Ubuntu
sudo apt install libsqlite3-dev

# Fedora/RHEL
sudo dnf install sqlite-devel
```

### OpenSSL 링킹 오류 (reqwest)
```bash
# Debian/Ubuntu
sudo apt install libssl-dev pkg-config

# Fedora/RHEL
sudo dnf install openssl-devel
```

### Raspberry Pi 크로스 컴파일 링커 오류
`~/.cargo/config.toml`에 링커 설정 추가:
```toml
[target.aarch64-unknown-linux-gnu]
linker = "aarch64-linux-gnu-gcc"
```

## 8. 빌드 스크립트

프로젝트에는 명령형/대화형 빌드 스크립트가 포함되어 있습니다.

### 8.1. 명령형 (비대화식 — CI/CD 용도)

| OS | 스크립트 | 예시 |
|----|----------|------|
| Windows | `build.bat` | `build.bat --test` |
| Linux/macOS | `build.sh` | `./build.sh --target rpi64` |

**옵션:**
- `--debug` — 디버그 빌드
- `--test` — 테스트만 실행
- `--clean` — 빌드 캐시 정리
- `--target <linux|rpi|rpi64|all>` — 크로스 빌드 (Linux/macOS only)

### 8.2. 대화형 (인터랙티브 — 개발자용)

| OS | 스크립트 |
|----|----------|
| Windows | `build_interactive.bat` |
| Linux/macOS | `build_interactive.sh` |

메뉴 선택 방식으로 빌드 타겟, 테스트, 린트 검사 등을 실행합니다.

## 9. CI/CD (GitHub Actions)

`.github/workflows/ci.yml` 파이프라인이 자동 실행됩니다.

### 파이프라인 단계

| 단계 | 내용 | 트리거 |
|------|------|--------|
| **Lint** | `cargo fmt --check` + `cargo clippy` | push, PR |
| **Test** | Windows + Linux 테스트 | push, PR |
| **Build** | Windows x64, Linux x64, RPi ARM64 릴리즈 빌드 | push, PR |
| **Release** | GitHub Release + 바이너리 첨부 | 태그 `v*` 푸시 |

### 릴리즈 생성 방법
```bash
git tag v0.4.0
git push origin v0.4.0
```
태그 푸시 시 CI가 자동으로 3개 타겟 바이너리를 빌드하고 GitHub Release에 첨부합니다.

