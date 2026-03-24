# femtoClaw

## 🇰🇷 한국어

### 개요
**femtoClaw** — 펨토초 수준의 압도적인 속도와 극단적인 메모리 최적화를 목표로 하는 TUI 기반 에이전트 자동화 플랫폼입니다. Rust로 작성된 단일 바이너리로, 라즈베리 파이부터 프로덕션 서버까지 어디에서나 실행 가능합니다.

### 주요 기능
- **⚙️ 에이전트 런타임:** OpenAI Function Calling 기반 도구 실행 + tiktoken-rs 토큰 카운터
- **💬 TUI 채팅:** 대시보드 내 분할 패널로 에이전트와 직접 대화 (background thread로 UI 비동기)
- **🌐 5개 언어 지원:** 한/영/일/중(번체)/중(간체) OS 언어 자동 감지
- **🔒 보안 샌드박스(Jailing):** 에이전트 파일 I/O가 `~/.femtoclaw/workspace/` 내부로 완벽히 격리
- **🔑 양방향 암호화:** ChaCha20Poly1305 기반 `config.enc`로 API 키 및 토큰 안전 보관
- **✅ 제로 컨피그 꼬임 방지:** API 키/토큰 저장 전 실제 HTTP 요청으로 유효성 검증
- **📱 텔레그램 연동:** 웹서버 없이 TUI PIN 페어링으로 즉시 연동
- **📦 정적 스킬 시스템:** TOML/JSON 기반 스킬 파일로 에이전트 기능 확장
- **⏪ Undo 지원:** SQLite WAL + ZSTD 압축 기반 최근 동작 되돌리기
- **🌐 7개 LLM 프리셋:** OpenAI, Gemini, Claude, xAI, OpenRouter, Ollama, LM Studio

### 지원 플랫폼
- Windows (x86_64)
- Linux (x86_64, aarch64)
- Raspberry Pi (ARM)

---

## 🇺🇸 English

### Overview
**femtoClaw** — A TUI-based agent automation platform targeting overwhelming speed at the femtosecond level and extreme memory optimization. Built as a single Rust binary, it runs anywhere from Raspberry Pi to production servers.

### Key Features
- **⚙️ Agent Runtime:** OpenAI Function Calling with tool execution + tiktoken-rs token counter
- **💬 TUI Chat:** Split-panel chat with the agent inside the Dashboard (non-blocking via background thread)
- **🌐 5 Languages:** Korean/English/Japanese/Chinese(Traditional)/Chinese(Simplified) with OS auto-detection
- **🔒 Secure Sandbox (Jailing):** Agent file I/O strictly isolated within `~/.femtoclaw/workspace/`
- **🔑 Bidirectional Encryption:** API keys and tokens safely stored via ChaCha20Poly1305-based `config.enc`
- **✅ Zero-Config Mess Prevention:** Validates API keys/tokens via actual HTTP requests before saving
- **📱 Telegram Integration:** Instant TUI PIN pairing without a web server
- **📦 Static Skill System:** Extend agent capabilities with TOML/JSON-based skill files
- **⏪ Undo Support:** Undo recent actions via SQLite WAL + ZSTD compression
- **🌐 7 LLM Presets:** OpenAI, Gemini, Claude, xAI, OpenRouter, Ollama, LM Studio

### Supported Platforms
- Windows (x86_64)
- Linux (x86_64, aarch64)
- Raspberry Pi (ARM)

---

## 🇯🇵 日本語

### 概要
**femtoClaw** — フェムト秒レベルの圧倒的な速度と極限的なメモリ最適化を目指すTUIベースのエージェント自動化プラットフォームです。Rust製の単一バイナリで、Raspberry Piから本番サーバまでどこでも実行可能です。

### 主な機能
- **⚙️ エージェントランタイム：** OpenAI Function Callingによるツール実行 + tiktoken-rsトークンカウンター
- **💬 TUIチャット：** ダッシュボード内の分割パネルでエージェントと直接会話（バックグラウンドスレッドでUI非同期）
- **🌐 5言語対応：** 韓/英/日/中(繁体)/中(簡体) OS言語自動検出
- **🔒 セキュアサンドボックス（Jailing）：** エージェントのファイルI/Oが `~/.femtoclaw/workspace/` 内に完全隔離
- **🔑 双方向暗号化：** ChaCha20Poly1305ベースの `config.enc` でAPIキーとトークンを安全保管
- **✅ ゼロ設定ミス防止：** 保存前にHTTPリクエストでAPIキー/トークンの有効性を検証
- **📱 Telegram連携：** Webサーバ不要、TUI PINペアリングで即時連携
- **📦 静的スキルシステム：** TOML/JSONベースのスキルファイルでエージェント機能を拡張
- **⏪ Undoサポート：** SQLite WAL + ZSTD圧縮ベースで最近の操作を元に戻す
- **🌐 7つのLLMプリセット：** OpenAI、Gemini、Claude、xAI、OpenRouter、Ollama、LM Studio

### 対応プラットフォーム
- Windows (x86_64)
- Linux (x86_64, aarch64)
- Raspberry Pi (ARM)

---

## 🇹🇼 繁體中文

### 概述
**femtoClaw** — 以飛秒級壓倒性速度和極致記憶體優化為目標的 TUI 代理自動化平台。以 Rust 單一二進位檔構建，從 Raspberry Pi 到生產伺服器皆可執行。

### 主要功能
- **⚙️ 代理運行時：** 基於 OpenAI Function Calling 的工具執行 + tiktoken-rs 令牌計數器
- **💬 TUI 聊天：** 儀表板內分割面板直接與代理對話（背景線程實現 UI 非同步）
- **🌐 5 種語言支援：** 韓/英/日/中(繁體)/中(簡體) OS 語言自動偵測
- **🔒 安全沙箱（Jailing）：** 代理檔案 I/O 完全隔離於 `~/.femtoclaw/workspace/` 內
- **🔑 雙向加密：** 透過 ChaCha20Poly1305 的 `config.enc` 安全儲存 API 金鑰和令牌
- **✅ 零配置混亂防護：** 儲存前透過實際 HTTP 請求驗證 API 金鑰/令牌有效性
- **📱 Telegram 整合：** 無需網路伺服器，透過 TUI PIN 配對即時連接
- **📦 靜態技能系統：** 透過 TOML/JSON 技能檔案擴展代理功能
- **⏪ Undo 支援：** 基於 SQLite WAL + ZSTD 壓縮的最近操作撤銷
- **🌐 7 個 LLM 預設：** OpenAI、Gemini、Claude、xAI、OpenRouter、Ollama、LM Studio

### 支援平台
- Windows (x86_64)
- Linux (x86_64, aarch64)
- Raspberry Pi (ARM)

---

## 🇨🇳 简体中文

### 概述
**femtoClaw** — 以飞秒级压倒性速度和极致内存优化为目标的 TUI 代理自动化平台。以 Rust 单一二进制文件构建，从 Raspberry Pi 到生产服务器均可执行。

### 主要功能
- **⚙️ 代理运行时：** 基于 OpenAI Function Calling 的工具执行 + tiktoken-rs 令牌计数器
- **💬 TUI 聊天：** 仪表板内分割面板直接与代理对话（后台线程实现 UI 非同步）
- **🌐 5 种语言支持：** 韩/英/日/中(繁体)/中(简体) OS 语言自动检测
- **🔒 安全沙箱（Jailing）：** 代理文件 I/O 完全隔离于 `~/.femtoclaw/workspace/` 内
- **🔑 双向加密：** 通过 ChaCha20Poly1305 的 `config.enc` 安全存储 API 密钥和令牌
- **✅ 零配置混乱防护：** 存储前通过实际 HTTP 请求验证 API 密钥/令牌有效性
- **📱 Telegram 集成：** 无需网络服务器，通过 TUI PIN 配对即时连接
- **📦 静态技能系统：** 通过 TOML/JSON 技能文件扩展代理功能
- **⏪ Undo 支持：** 基于 SQLite WAL + ZSTD 压缩的最近操作撤销
- **🌐 7 个 LLM 预设：** OpenAI、Gemini、Claude、xAI、OpenRouter、Ollama、LM Studio

### 支持平台
- Windows (x86_64)
- Linux (x86_64, aarch64)
- Raspberry Pi (ARM)

---

## 빌드 및 실행 (Build & Run)

```bash
# 빌드
cargo build --release

# 실행 (TUI 모드)
./target/release/femtoclaw

# 실행 (헤드리스/백그라운드 모드)
./target/release/femtoclaw --headless
```

## 라이선스 (License)

TBD
