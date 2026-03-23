# i18n 리팩토링 전수 조사 및 마이그레이션 가이드

## 프로젝트: femtoClaw v0.4.0 → v0.5.0
## 최종 갱신: 2026-03-24
## 대상 언어: 한국어(ko) / 영어(en) / 일본어(ja) / 중국어 번체(zh-tw) / 중국어 간체(zh-cn)

---

## 1. 전수 조사 결과

### 1.1. 전체 규모

| 항목 | 수치 |
|------|------|
| 하드코딩 한국어 포함 라인 | **356줄** |
| 해당 소스 파일 | **18개** |
| 고유 i18n 키 (중복 제거 추정) | **~220개** |

### 1.2. 파일별 분포 (심각도 순)

| 파일 | 라인수 | 분류 | 비고 |
|------|--------|------|------|
| `src/db/store.rs` | 63 | DB/테스트 | 대부분 테스트 assert 메시지 + display_name() |
| `src/tui/app.rs` | 50 | **TUI** | 사용자 대면 문자열 최다 — 화면별 치환 필요 |
| `src/tools/registry.rs` | 31 | 도구 명세 | tool name, description, constraints, error_guidance |
| `src/tools/guide.rs` | 26 | Jailing 안내 | 사용자 안내 메시지 전량 |
| `src/skills/loader.rs` | 26 | 스킬 엔진 | 에러 메시지 + 로그 |
| `src/skills/rhai_engine.rs` | 21 | Rhai 엔진 | 에러 메시지 |
| `src/tools/executor.rs` | 20 | 도구 실행기 | 보안 차단 메시지 + 에러 |
| `src/tools/prompt.rs` | 19 | 프롬프트 빌더 | LLM system prompt (대형 텍스트 블록) |
| `src/core/telegram.rs` | 19 | 텔레그램 봇 | 봇 응답 메시지 |
| `src/main.rs` | 17 | 진입점 | headless 로그 + CLI 메시지 |
| `src/core/agent.rs` | 13 | 에이전트 | 에러 메시지 |
| `src/security/jail.rs` | 11 | 보안 | 에러 메시지 |
| `src/error.rs` | 10 | 에러 타입 | thiserror #[error("...")] 메시지 |
| `src/security/crypto.rs` | 10 | 암호화 | 에러/로그 |
| `src/sandbox.rs` | 7 | 샌드박스 | 에러 메시지 |
| `src/config.rs` | 7 | 설정 | 에러 메시지 |
| `src/db/compress.rs` | 3 | ZSTD 압축 | 에러 메시지 |
| `src/core/agent_manager.rs` | 3 | 에이전트 관리 | 에러 메시지 |

---

## 2. 문자열 분류

### Category A: TUI 화면 렌더링 (언어별 화면 치환)
> **전략: 언어별 별도 렌더 함수**. 폭 계산 없이 화면 자체를 통째로 바꾼다.

| 위치 | 키 ID | 한국어 원문 | 비고 |
|------|--------|-------------|------|
| app.rs:194 | `boot.init_msg` | `femtoClaw v0.1.0-beta 시작` | feed 초기값 |
| app.rs:237 | `feed.llm_verify_ok` | `LLM 검증 성공: {} — {}개 모델 발견` | format! |
| app.rs:244 | `feed.llm_verify_ok_simple` | `LLM 검증 성공: {}` | |
| app.rs:251 | `feed.llm_verify_fail` | `LLM 검증 실패: {}` | |
| app.rs:256 | `feed.tg_verify_ok` | `Telegram 검증 성공: {}` | |
| app.rs:261 | `feed.tg_verify_fail` | `Telegram 검증 실패: {}` | |
| app.rs:370 | `dash.agent_status` | `━━ Agent Status ━━` | |
| app.rs:372 | `dash.agent_name` | `에이전트: {}` | |
| app.rs:375 | `dash.model` | `모델: {} ({:?})` | |
| app.rs:378 | `dash.security` | `보안: Jailing=ON \| ChaCha20=ON` | |
| app.rs:391 | `dash.llm_none` | `LLM 미설정` | |
| app.rs:397 | `dash.skill_header` | `━━ Skill List (TOML + Rhai) ━━` | |
| app.rs:409 | `dash.skill_builtin` | `[내장][{}] {} — {}` | format! |
| app.rs:412 | `dash.skill_core_fail` | `core 로드 실패: {}` | |
| app.rs:422 | `dash.skill_user` | `[사용자][{}] {} — {}` | format! |
| app.rs:427 | `dash.skill_user_fail` | `user 로드 실패: {}` | |
| app.rs:433 | `dash.timemachine_header` | `━━ Time Machine (최근 10건) ━━` | |
| app.rs:436 | `dash.timemachine_cols` | `유형 / 상태 / 요약 / 시각` | 컬럼 헤더 |
| app.rs:444 | `dash.no_records` | `(기록 없음)` | |
| app.rs:447 | `dash.undo_label` | `↩ Undo / ✅ 완료` | |
| app.rs:464 | `dash.total_count` | `── 전체 {} 건 ──` | |
| app.rs:470 | `dash.db_query_fail` | `DB 조회 실패: {}` | |
| app.rs:474 | `dash.db_open_fail` | `DB 열기 실패: {}` | |
| app.rs:481 | `dash.agent_switch_header` | `━━ Agent Switch ━━` | |
| app.rs:483 | `dash.no_agents` | `(등록된 에이전트 없음)` | |
| app.rs:491 | `dash.agent_active` | `활성 / 비활성` | |
| app.rs:519 | `dash.agent_switched` | `→ 에이전트 #{}({})로 전환!` | |
| app.rs:523 | `dash.no_switch` | `(전환 가능한 다른 에이전트 없음)` | |
| app.rs:534 | `dash.agent_added` | `✅ 에이전트 #{} ({}) 추가 완료` | |
| app.rs:541 | `dash.agent_add_fail` | `❌ {}` | |
| app.rs:555 | `pw.empty` | `비밀번호를 입력하세요` | |
| app.rs:559 | `pw.too_short` | `최소 4자 이상 입력하세요` | |
| app.rs:563 | `pw.mismatch` | `비밀번호가 일치하지 않습니다` | |
| app.rs:574 | `pw.key_generated` | `마스터 키 생성 완료` | |
| app.rs:578 | `pw.save_fail` | `설정 저장 실패: {}` | |
| app.rs:587 | `pw.decrypt_ok` | `설정 복호화 성공` | |
| app.rs:593 | `pw.3fail_reset` | `3회 실패. [R]을 눌러 설정을 리셋하세요` | |
| app.rs:595 | `pw.wrong_pw` | `비밀번호 오류 ({}/3)` | |
| app.rs:710 | `onboard.save_ok` | `설정 저장 완료 → 대시보드` | |
| app.rs:715 | `onboard.save_fail` | `❌ 설정 저장 실패: {} — 디스크 용량/권한을 확인하세요` | |
| app.rs:918 | `onboard.llm_status_wait` | `검증 대기` | |
| app.rs:921 | `onboard.llm_status_testing` | `검증 중 (최대 5초)` | |
| app.rs:929 | `onboard.llm_status_fail_retry` | `[V]로 재시도` | |
| app.rs:990 | `onboard.tg_status_wait` | `검증 대기 (선택사항)` | |
| app.rs:994 | `onboard.tg_status_testing` | `검증 중 (최대 5초)` | |
| app.rs:998 | `onboard.tg_status_ok` | `Telegram Bot 확인됨` | |
| app.rs:1002 | `onboard.tg_status_fail_retry` | `[V]로 재시도` | |

### Category B: 에러 메시지 (키 단위 치환)
> **전략: msg!(key, args...)** 매크로로 치환.

| 위치 | 키 ID | 한국어 원문 |
|------|--------|-------------|
| error.rs:13 | `err.home_not_found` | `홈 디렉토리를 찾을 수 없습니다` |
| error.rs:17 | `err.sandbox_create` | `샌드박스 디렉토리 생성 실패: {}` |
| error.rs:21 | `err.already_running` | `femtoClaw가 이미 실행 중입니다 (PID: {})` |
| error.rs:25 | `err.lock_file` | `락 파일 처리 실패: {}` |
| error.rs:30 | `err.key_derivation` | `암호화 키 파생 실패` |
| error.rs:34 | `err.encryption` | `데이터 암호화 실패` |
| error.rs:38 | `err.decryption` | `복호화 실패: 비밀번호가 올바르지 않거나 데이터가 손상되었습니다` |
| error.rs:43 | `err.config_io` | `설정 파일 I/O 오류: {}` |
| error.rs:47 | `err.invalid_config` | `설정 파일 형식이 올바르지 않습니다` |
| error.rs:51 | `err.serialization` | `설정 직렬화 오류: {}` |
| config.rs:138 | `err.max_agents` | `에이전트는 최대 3개까지 등록 가능합니다` |
| agent.rs (다수) | `err.api_*` | API 호출 관련 에러 13건 |
| jail.rs (다수) | `err.jail_*` | Jailing 관련 에러 11건 |
| crypto.rs (다수) | `err.crypto_*` | 암호화 관련 에러 10건 |
| sandbox.rs (다수) | `err.sandbox_*` | 샌드박스 에러 7건 |

### Category C: 도구 하네스 (도구 명세 + LLM 프롬프트)
> **전략: 키 단위 치환** — 다만 BUILTIN_TOOLS는 const이므로 lazy_static 또는 함수 반환으로 변경 필요.

| 위치 | 키 ID | 한국어 원문 | 비고 |
|------|--------|-------------|------|
| registry.rs:25 | `tool.level.safe` | `안전` | SecurityLevel::display() |
| registry.rs:26 | `tool.level.jail` | `Jail 검증` | |
| registry.rs:27 | `tool.level.restricted` | `제한됨` | |
| registry.rs:72 | `tool.param.path_desc` | `workspace 내 상대 경로` | |
| registry.rs:80 | `tool.param.content_desc` | `파일에 쓸 내용` | |
| registry.rs:81 | `tool.param.content_ex` | `분석 결과: 정상` | |
| registry.rs:88 | `tool.param.dir_desc` | `workspace 내 디렉토리 상대 경로` | |
| registry.rs:96 | `tool.param.ms_desc` | `대기 시간 (밀리초, 최대 5000)` | |
| registry.rs:104 | `tool.param.msg_desc` | `출력할 메시지` | |
| registry.rs:105 | `tool.param.msg_ex` | `처리 완료!` | |
| registry.rs:113 | `tool.file_read.name` | `파일 읽기` | |
| registry.rs:115 | `tool.file_read.desc` | `workspace 내 파일을 읽어...` | |
| registry.rs:117-118 | `tool.file_read.constraints` | `경로는 반드시 workspace/ 내부...` | |
| registry.rs:119-120 | `tool.file_read.err_guide` | `파일이 없으면 사용자에게...` | |
| registry.rs:124 | `tool.file_write.name` | `파일 쓰기` | |
| registry.rs:126-127 | `tool.file_write.desc` | `workspace 내에 파일을 생성...` | |
| registry.rs:129-131 | `tool.file_write.constraints` | `경로는 반드시...` | |
| registry.rs:132-133 | `tool.file_write.err_guide` | `쓰기 실패 시...` | |
| registry.rs:137 | `tool.file_list.name` | `디렉토리 목록` | |
| registry.rs:148 | `tool.sleep.name` | `대기` | |
| registry.rs:150 | `tool.sleep.desc` | `지정한 시간 동안...` | |
| registry.rs:159 | `tool.print.name` | `로그 출력` | |
| registry.rs:161 | `tool.print.desc` | `메시지를 출력 버퍼에...` | |

### Category D: LLM 시스템 프롬프트 (대형 텍스트 블록)
> **전략: 언어별 const 텍스트 블록**. TUI와 같은 원리 — 통째로 치환.

| 위치 | 키 ID | 성격 |
|------|--------|------|
| prompt.rs:13-32 | `prompt.jailing_section` | Jailing 안내 (15줄 블록) |
| prompt.rs:35-43 | `prompt.error_rules` | 에러 처리 규칙 (8줄 블록) |
| prompt.rs:53-58 | `prompt.agent_identity` | 에이전트 정체성 소개 |
| prompt.rs:62 | `prompt.tools_header` | `사용 가능한 도구` |
| prompt.rs:69 | `prompt.params_header` | `파라미터:` |
| prompt.rs:71 | `prompt.param_required` | `필수 / 선택` |
| prompt.rs:81 | `prompt.constraint_label` | `제약:` |
| prompt.rs:84 | `prompt.error_label` | `에러 시:` |

### Category E: Jailing 가이드 (사용자 안내 메시지)
> **전략: 함수 반환값을 locale별 분기.**

| 위치 | 키 ID | 한국어 원문 | 비고 |
|------|--------|-------------|------|
| guide.rs (26건) | `guide.welcome` | 웰컴 메시지 | |
| guide.rs | `guide.blocked_explain` | 차단 설명 메시지 | |
| guide.rs | `guide.workspace_info` | workspace 안내 | |
| guide.rs | `guide.error_help` | 에러 도움 메시지 | |

### Category F: 텔레그램 봇 응답 (봇 메시지)
> **전략: 키 단위 치환.**

| 위치 | 키 ID | 한국어 원문 |
|------|--------|-------------|
| telegram.rs (19건) | `bot.pair_prompt` | PIN 입력 안내 |
| telegram.rs | `bot.pair_success` | 페어링 성공 |
| telegram.rs | `bot.pair_fail` | PIN 불일치 |
| telegram.rs | `bot.help` | 명령어 도움말 |
| telegram.rs | `bot.agent_switch` | 에이전트 전환 안내 |

### Category G: DB 레이어 (display_name + 테스트)
> **전략: display_name()에 locale 파라미터 추가. 테스트 assert문은 키 대신 영어 기본값 사용.**

| 위치 | 키 ID | 한국어 원문 | 비고 |
|------|--------|-------------|------|
| store.rs | `db.type.user_msg` | `사용자 메시지` | ActionType::display_name() |
| store.rs | `db.type.agent_resp` | `에이전트 응답` | |
| store.rs | `db.type.file_op` | `파일 작업` | |
| store.rs | `db.type.config_change` | `설정 변경` | |
| store.rs | `db.type.tool_call` | `도구 호출` | |
| store.rs | `db.type.security_event` | `보안 이벤트` | |
| store.rs (테스트) | - | assert 메시지 ~40건 | 테스트 전용 — i18n 제외 가능 |

### Category H: Headless/CLI 메시지
> **전략: 키 단위 치환.**

| 위치 | 키 ID | 한국어 원문 |
|------|--------|-------------|
| main.rs:94 | `cli.headless_banner` | `femtoClaw v0.4.0 — Headless Mode` |
| main.rs:100 | `cli.no_config` | `config.enc가 없습니다...` |
| main.rs:111 | `cli.enter_pw` | `마스터 비밀번호:` |
| main.rs:122 | `cli.no_telegram` | `텔레그램 설정이 없거나 미검증...` |
| main.rs:148 | `cli.paired` | `페어링 성공: {} (chat_id: {})` |
| main.rs:155+ | `cli.msg_received` 등 | 이벤트 로그 7건 |

---

## 3. i18n 아키텍처 설계

### 3.1. 핵심 전략: 하이브리드 (TUI 치환 + 키 치환)

```
src/i18n/
├── mod.rs          → Lang enum, 현재 locale, msg!() 매크로
├── keys.rs         → 메시지 키 상수 정의
├── ko.rs           → 한국어 메시지 맵 (기본값)
├── en.rs           → 영어 메시지 맵
├── ja.rs           → 일본어 메시지 맵
├── zh_tw.rs        → 중국어 번체 메시지 맵
└── zh_cn.rs        → 중국어 간체 메시지 맵
```

### 3.2. TUI 화면 치환 전략

TUI 렌더링 함수는 **언어별 별도 구현**. CJK 폭 계산 대신 화면 자체를 바꾼다:

```rust
// tui/app.rs — 렌더링 디스패치
fn render_password(&self, frame: &mut Frame) {
    match current_lang() {
        Lang::Ko => self.render_password_ko(frame),
        Lang::En => self.render_password_en(frame),
        Lang::Ja => self.render_password_ja(frame),
        Lang::ZhTw => self.render_password_zh_tw(frame),
        Lang::ZhCn => self.render_password_zh_cn(frame),
    }
}
```

TUI 화면은 4개 (Boot, Password, Onboard, Dashboard) × 5개 언어 = 최대 20개.
단, Boot/Password/Onboard는 유사할 수 있으므로 공통 로직과 텍스트만 분리하여 실제 추가 함수 수를 최소화.

### 3.3. 메시지 키 치환 (비-TUI)

```rust
// 사용법
msg!("err.home_not_found")              // 단순 키
msg!("err.sandbox_create", err)         // format 인자 포함
msg!("cli.paired", username, chat_id)   // 다중 인자
```

### 3.4. 테스트 코드 처리

- assert 메시지 내 한국어 → **i18n 적용 제외**
- 주석 내 한국어 → D3D 룰에 따라 한국어 유지 (i18n 대상 아님)
- `assert_eq!(tool.name, "파일 읽기")` → 키 기반으로 변경 시 테스트도 수정 필요

### 3.5. const 제약 해결

`BUILTIN_TOOLS`는 `const`라서 `msg!()` 호출 불가.
→ `fn builtin_tools(lang: Lang) -> Vec<ToolDef>` 함수로 변경하거나,
→ `id`는 영어 고정, `name/description`만 별도 조회 함수로 분리.

---

## 4. 마이그레이션 체크리스트

### Phase 1: 인프라 구축 (v0.5.0-alpha)
- [ ] `src/i18n/mod.rs` — Lang enum, CURRENT_LANG 전역, msg!() 매크로
- [ ] `src/i18n/keys.rs` — 메시지 키 상수 (~220개)
- [ ] `src/i18n/ko.rs` — 한국어 기본 맵 (현 하드코딩 내용 그대로)
- [ ] `src/i18n/en.rs` — 영어 맵 (Phase 2에서 채움, stub만)
- [ ] 기본 테스트 (msg! 동작, 언어 전환, fallback)
- [ ] main.rs에 `--lang` CLI 인자 추가

### Phase 2: 사용자 대면 문자열 마이그레이션
- [ ] tui/app.rs — TUI 렌더링 함수 언어별 분리
- [ ] error.rs — thiserror 메시지 → msg!()
- [ ] main.rs — CLI/headless 메시지 → msg!()
- [ ] en.rs, ja.rs, zh_tw.rs, zh_cn.rs 번역 채우기

### Phase 3: 내부 모듈 마이그레이션
- [ ] tools/registry.rs — ToolDef name/description → 함수 반환
- [ ] tools/prompt.rs — LLM 프롬프트 블록 → 언어별 const
- [ ] tools/guide.rs — Jailing 안내 → msg!()
- [ ] tools/executor.rs — 보안 메시지 → msg!()
- [ ] core/telegram.rs — 봇 응답 → msg!()

### Phase 4: 2차 모듈 + 최종 검증
- [ ] db/store.rs — display_name() → msg!()
- [ ] skills/ — 에러 메시지 → msg!()
- [ ] security/ — 에러 메시지 → msg!()
- [ ] config.rs — 에러 메시지 → msg!()
- [ ] 전체 grep 재스캔: 누락 한국어 0건 확인
- [ ] 5개 언어 전체 TUI 스크린샷 검증

---

## 5. 주의사항

1. **테스트 깨짐 위험**: `assert_eq!(tool.name, "파일 읽기")` 같은 테스트는 키 변경 시 동시 수정
2. **const 제약**: Rust const에서 함수 호출 불가 → `BUILTIN_TOOLS`는 함수로 전환
3. **thiserror 제약**: `#[error("...")]`은 컴파일 타임 문자열 → Display impl 수동 구현으로 변경
4. **텔레그램 언어 감지**: 봇-사이드는 Telegram API의 language_code로 자동 감지 가능
5. **바이너리 크기**: 5개 언어 × 220키 × 평균 50바이트 ≈ 55KB — 무시 가능
6. **TUI 화면 중복**: 화면별로 언어 치환하지만, 레이아웃 로직은 공통 함수로 추출하여 중복 최소화
