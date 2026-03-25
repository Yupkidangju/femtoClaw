# femtoClaw 빌드 가이드

## 사전 요구 사항

| 항목 | 최소 버전 | 설치 |
|------|----------|------|
| Rust | 1.70+ | [rustup.rs](https://rustup.rs) |
| Git | 2.0+ | 패키지 매니저 |

## 빌드

### 디버그 빌드 (개발)
```bash
cargo build
```

### 릴리즈 빌드 (최적화)
```bash
cargo build --release
```
결과: `target/release/femtoclaw` (Linux/macOS) 또는 `target/release/femtoclaw.exe` (Windows)

## 크로스 컴파일

### Raspberry Pi (ARM)
```bash
rustup target add armv7-unknown-linux-gnueabihf
cargo build --release --target armv7-unknown-linux-gnueabihf
```

### Raspberry Pi 64-bit (aarch64)
```bash
rustup target add aarch64-unknown-linux-gnu
cargo build --release --target aarch64-unknown-linux-gnu
```

### Linux (x86_64)
```bash
rustup target add x86_64-unknown-linux-gnu
cargo build --release --target x86_64-unknown-linux-gnu
```

## 테스트

### 단위 + 라이브러리 테스트
```bash
cargo test
```

### 통합 테스트만
```bash
cargo test --test simulation_30turn
```

## 실행

### TUI 모드 (기본)
```bash
./femtoclaw
```

### 헤드리스 모드 (텔레그램 전용)
```bash
./femtoclaw --headless
```

### 스케줄러 모드
```bash
./femtoclaw --run-schedule          # 내장 스케줄러 실행
./femtoclaw --install-schedule      # OS 예약 등록
./femtoclaw --uninstall-schedule    # OS 예약 해제
```

### 언어 오버라이드
```bash
./femtoclaw --lang ko    # 한국어
./femtoclaw --lang en    # English
./femtoclaw --lang ja    # 日本語
./femtoclaw --lang zh_tw # 繁體中文
./femtoclaw --lang zh_cn # 简体中文
```

## systemd (Linux 서비스)
```bash
# 서비스 파일 복사
sudo cp docs/femtoclaw.service /etc/systemd/system/
# 서비스 활성화 + 시작
sudo systemctl enable femtoclaw
sudo systemctl start femtoclaw
# 상태 확인
sudo systemctl status femtoclaw
```

## 디렉토리 구조

설치 후 최초 실행 시 자동 생성:
```
~/.femtoclaw/
├── .lock              # 프로세스 락
├── config.enc         # 암호화 설정
├── db/femto_state.db  # SQLite DB
├── skills/            # 스킬 파일
└── workspace/         # 에이전트 작업 공간
```
