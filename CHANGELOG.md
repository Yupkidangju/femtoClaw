# 변경 이력 (Changelog)

이 프로젝트의 모든 주요 변경사항은 이 파일에 기록됩니다.
형식은 [Keep a Changelog](https://keepachangelog.com/ko/1.1.0/)를 따르며,
버전 관리는 [Semantic Versioning](https://semver.org/spec/v2.0.0.html)을 준수합니다.




## [0.7.0] - 2026-03-24

### 추가됨 (Added)
- **ChatWorker** — background thread + mpsc 채널 기반 비동기 LLM 호출 (TUI가 LLM 응답 대기 중에도 멈추지 않음)
- **ChatEvent** enum — Thinking / Reply / ToolUsed / Error (tick()에서 비동기 수신)
- **TokenState** — thread-safe Arc<Mutex> 기반 토큰 상태 공유
- **입력 잠금** — LLM 응답 대기 중(chat_waiting) 키 입력 자동 차단 + "🤔 Thinking..." 표시
- **MEMORY.md 자동 큐레이션** — 매 대화 후 요약 라인 추가, 100줄 초과 시 FIFO 자동 정리
- **세션 트랜스크립트** — `sessions/YYYYMMDD_HHMMSS.md` 형식으로 전체 대화 자동 기록
- **CJK 레이아웃 보정** — unicode-width 크레이트 적용, truncate_for_display() 표시 너비 기준으로 변경
- **텔레그램 다국어** — 봇 메시지 4건(pair_success/fail/prompt, help) msg!() 전환

### 변경됨 (Changed)
- Cargo.toml 버전: 0.6.0 → **0.7.0**
- TUI: `ChatSession` 직접 호출(blocking) → `ChatWorker` 비동기 전환
- Dashboard 토큰 표시: `session.token_usage()` → `worker.token_state()` (thread-safe)

### 참고사항
- v0.6.0의 최대 이슈(TUI blocking 5~30초) 완전 해소
- 110개 테스트 전체 통과

## [0.6.0] - 2026-03-24

### 추가됨 (Added)
- **[Phase 1] Bootstrap + Persona** — agent.toml/user.toml/MEMORY.md 자동 생성, 첫 실행 의식(BOOTSTRAP.md)
- **[Phase 2] Chat Loop** — handle_message() 단일 함수로 TUI/Telegram 공용 대화 루프 구현
- **[Phase 2] Function Calling** — OpenAI tools 파라미터 + tool_calls 응답 파싱 + executor 연쇄 (최대 5 라운드)
- **[Phase 2] Token Counter** — tiktoken-rs (cl100k_base) 기반 정확한 토큰 수 산정, 컨텍스트 윈도우 자동 트림
- **[Phase 2] Context Manager** — persona + tools + MEMORY.md + daily_log → system prompt 조립
- **[Phase 3] Bootstrap 통합** — main.rs run()에 자동 bootstrap 호출 (workspace에 agent.toml 없으면 초기화)
- **[Phase 4] TUI Chat Panel** — Dashboard [C] 키로 채팅 모드 진입, 상단(대화기록)+하단(입력) 분할, 토큰 사용량 실시간 표시
- **[Phase 4] Telegram 연동** — headless 모드 MessageReceived → chat_loop → SendResponse (TODO 해소)
- **core/persona.rs, core/bootstrap.rs, core/context.rs, core/tool_protocol.rs, core/chat_loop.rs** 신규 모듈
- **tiktoken-rs = "0.9"** 의존성 추가

### 변경됨 (Changed)
- Cargo.toml 버전: 0.4.0 → **0.6.0**
- core/agent.rs 전면 리팩토링 — ChatMessage에 tool_call_id/tool_calls/name 필드 추가, AgentResponse에 tool_calls 파싱
- main.rs run_headless() — TODO 주석을 실제 chat_loop 호출로 대체
- spec.md §3.5a Agent Runtime Architecture 섹션 추가
- dashboard 헤더에 토큰 사용량 표시, 상태바에 채팅 모드 안내

### 참고사항
- **v0.6.0 전체 완료** — Phase 1~4, 110개 테스트 통과
- 핵심 철학: "인프라에 두뇌를 입주시키다" — agent.rs/executor/prompt/registry가 chat_loop을 통해 전부 연결됨
- 에이전트가 "살아있는 봇"으로 동작 가능한 최초 버전

## [0.4.0] - 2026-03-23

### 추가됨 (Added)
- **[Step 9a] Tool Registry** — ToolDef, ToolParam, SecurityLevel 구조체 + BUILTIN_TOOLS 5종 등록
- **[Step 9b] System Prompt Builder** — LLM system message에 도구 명세 + Jailing 개념 + 에러 규칙 자동 주입
- **[Step 9c] Safe Tool Executor** — 검증 → Jailing → 실행 → 에러 복구 파이프라인, 3회 실패 자동 중단
- **[Step 9d] Jailing Guide** — 사용자가 Jailing을 몰라도 에이전트가 친절하게 안내하는 메시지 시스템
- **[Step 9e] Tool Audit** — ActionType에 ToolCall + SecurityEvent 추가, DB 감사 기록 지원
- **src/tools/ 모듈** — registry.rs, prompt.rs, executor.rs, guide.rs

### 변경됨 (Changed)
- Cargo.toml 버전: 0.3.0 → **0.4.0**
- spec.md §13 Tool Harness 상세 사양 추가
- DB ActionType 확장: ToolCall(도구), SecurityEvent(보안)

### 참고사항
- **v0.4.0 전체 완료** — Step 9a~9e, 74개 테스트 통과
- 핵심 철학: "도구가 있으면 거기에 잘 태워야 한다"

## [0.3.0] - 2026-03-23

### 추가됨 (Added)
- **[Step 8a] AgentConfig** — 에이전트별 LLM 설정 모델 (최대 3개)
- **[Step 8a] AppConfig 확장** — agents Vec, active_agent_id, add_agent(), switch_agent()
- **[Step 8b] AgentPaths** — 에이전트별 격리 DB + workspace 디렉토리 자동 생성
- **[Step 8b] AgentManager** — 멀티 에이전트 경로 관리
- **[Step 8c] 텔레그램 라우팅** — /agent N (전환), /agents (목록), 세션별 활성 에이전트
- **[Step 8d] TUI [5] Agent Switch** — 순환 전환
- **[Step 8d] TUI [A] Add Agent** — 대시보드에서 에이전트 추가

### 변경됨 (Changed)
- Cargo.toml 버전: 0.2.0 → **0.3.0**
- spec.md §12 멀티 에이전트 상세 사양 추가
- BotState에 active_agent_id + agent_ids 추가
- SandboxPaths 확장 (v0.2 기존 유지)

### 참고사항
- **v0.3.0 전체 완료** — Step 8a~8d, 53개 테스트 통과
- 하위 호환: v0.1/v0.2 설정 파일은 자동으로 에이전트 #1에 매핑

## [0.2.0] - 2026-03-23

### 추가됨 (Added)
- **[Step 6a] Rhai 동적 스킬 엔진** — sandboxed 실행 (ops 100만, stack 32, string 1MB 제한)
- **[Step 6a] 호스트 함수 5종** — file_read, file_write, file_list, sleep, print
- **[Step 6b] 하이브리드 스킬 로더** — .toml + .rhai 공존, @name/@desc 메타데이터 파싱
- **[Step 6c] TUI 스킬 실행기** — 대시보드 [3]에서 내장/사용자 스킬 목록 표시
- **[Step 7a] DB 쿼리 확장** — 페이지네이션, 유형별 필터, 선택적 Undo, SkillRun 유형
- **[Step 7b] TUI 타임머신** — 대시보드 [4]에서 전체 이력 테이블 뷰
- **[Step 7c] 선택적 Undo** — undo_by_id()로 임의의 과거 액션 Undo
- **예제 Rhai 스킬** — skills/core/auto_summarize.rhai

### 변경됨 (Changed)
- Cargo.toml 버전: 0.1.0-beta → **0.2.0**
- spec.md v0.2 상세 사양 추가 (§10~§11)
- DB 스키마 버전: 1 → **2** (action_type 인덱스 추가)
- SandboxPaths에 skills_core, skills_user, db_dir 경로 추가
- 대시보드 메뉴: [3] Skills (TOML+Rhai), [4] Time Machine

### 참고사항
- **v0.2.0 전체 완료** — Step 6a~6c + 7a~7c, 48개 테스트 통과
- 다음 버전: **v0.3 — 멀티 에이전트 (최대 3개)**

## [0.1.0-beta] - 2026-03-23

### 추가됨 (Added)
- 프로젝트 초기 스펙 문서(`spec.md`) 작성 완료
- 인터페이스 디자인 청사진(`designs.md`) 작성 완료
- 디자인 방향 확정: **Design 2 — Amber Monochrome** (Midnight Commander 스타일)
- D3D Protocol 기반 전체 문서 체계 수립 (9개 필수 문서)
- LLM Provider 전략 확립: 2-Format (OpenAI-Compatible + Ollama), 7개 프리셋
- 정적 파일 기반 스킬 시스템 설계 (TOML/JSON)
- Headless 모드 상세 사양 추가
- 에러/장애 복구 시나리오 정의 (Exponential Backoff, DB 복구, 오프라인 큐잉)
- 비밀번호 3회 실패 리셋 정책 추가
- 버전 로드맵 정의 (v0.1 → v0.2 Rhai → v0.3 멀티 에이전트)
- **[Step 1] 코어 샌드박스 초기화** — 디렉토리 구조, 프로세스 락, config.enc 암호화/복호화
- **[Step 2] TUI 온보딩 & 검증** — Amber Monochrome 테마, 4개 화면(Boot→PW→Onboard→Dashboard)
- **[Step 2] API 키 비동기 검증** — reqwest 별도 스레드 + mpsc 채널 (TUI 비블로킹)
- **[Step 2] 모델 자동 선택기** — /models 파싱 → ↑↓ 방향키로 모델 순환 선택
- **[Step 2] 입력 방어 로직** — 글자 수 상한(PW 128, Key 256, Model 64, Token 128) + 표시 잘림
- **[Step 2] Windows KeyEvent 필터** — `KeyEventKind::Press`만 처리 (Release/Repeat 무시)
- **[Step 2] 대시보드 메뉴 스텁** — [1]-[4] 키 피드백 응답
- **[Step 3] SQLite WAL + ZSTD 압축 DB** — 에이전트 액션 저장, Undo, 무결성 검사, 백업/복구
- **[Step 4] 텔레그램 봇** — teloxide Long-Polling, PIN 페어링, LLM 에이전트 클라이언트
- **[Step 4] Exponential Backoff** — 1→2→4→...→60초 재시도, 5분 경고 플래그
- **[Step 4] 오프라인 큐잉** — 네트워크 끊김 시 메시지 대기열 (최대 크기 제한)
- **[Step 4] Headless 모드** — config.enc 로드 → 봇 시작 → 이벤트 루프
- **[Step 4] Graceful Shutdown** — ctrlc + AtomicBool + shutdown_token 연동
- **[Step 5] Path Jailing** — workspace 강제 제한, ../ 순회 차단, symlink 검증
- **[Step 5] 블랙리스트** — 20개 파괴 명령어 필터링
- **[Step 5] TOML 스킬 시스템** — core/user 디렉토리 스캔, 저장/로드
- **[Step 5] 내장 스킬 3종** — 파일 읽기, 코드 리뷰, 요약 어시스턴트
- **빌드 스크립트** — Windows/Linux 명령형 + 대화형 (build.bat/.sh, build_interactive.bat/.sh)
- **CI/CD** — GitHub Actions 4단계 파이프라인 (Lint → Test → Build Win+Linux → Release)

### 변경됨 (Changed)
- spec.md 전면 개정: v0.1 경량화 방향 반영
- designs.md 전면 개정: Design 2 기준, 단일 에이전트 UI, 간소 Undo
- 에이전트 수: 최대 3개 → **1개** (v0.1)
- 타임머신: 풀 데이터 그리드 → **간소 Undo** (최근 5건 + 마지막 취소)
- 부트 시퀀스: 매번 풀 연출 → **빠른 부트 기본** (0.5초)
- LLM 공급자: 7개 개별 구현 → **2개 API 형식 + 7개 프리셋**
- **HTTP 타임아웃: 10초 → 5초** (connect 3초 + total 5초)
- **검증 재시도: Testing 중에도 [V] 재입력 허용**

### 제거됨 (Removed)
- Rhai 동적 스크립팅 엔진 (v0.2 이후 검토)
- 멀티 에이전트 라우팅 로직 (v0.3 이후 검토)

### 참고사항
- **v0.1.0 전체 완료** — Step 1~5 + CI/CD, 37개 테스트 통과
- RPi는 네이티브 빌드 권장 (rusqlite bundled + zstd C-binding 크로스컴파일 제한)
- 다음 버전: **v0.2 — Rhai 동적 스킬 엔진 + 풀 타임머신 UI**


