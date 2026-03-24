// femtoClaw — 한국어 메시지 맵
// [v0.5.0] 기본 언어. 현재 하드코딩된 모든 한국어 문자열의 원본.

/// 한국어 메시지 조회
pub fn get(key: &str) -> Option<&'static str> {
    Some(match key {
        // === 에러 ===
        "err.home_not_found" => "홈 디렉토리를 찾을 수 없습니다",
        "err.sandbox_create" => "샌드박스 디렉토리 생성 실패: {}",
        "err.already_running" => "femtoClaw가 이미 실행 중입니다 (PID: {})",
        "err.lock_file" => "락 파일 처리 실패: {}",
        "err.key_derivation" => "암호화 키 파생 실패",
        "err.encryption" => "데이터 암호화 실패",
        "err.decryption" => "복호화 실패: 비밀번호가 올바르지 않거나 데이터가 손상되었습니다",
        "err.config_io" => "설정 파일 I/O 오류: {}",
        "err.invalid_config" => "설정 파일 형식이 올바르지 않습니다",
        "err.serialization" => "설정 직렬화 오류: {}",
        "err.max_agents" => "에이전트는 최대 3개까지 등록 가능합니다",
        "err.http_client" => "HTTP 클라이언트 생성 실패: {}",

        // === 비밀번호 ===
        "pw.empty" => "비밀번호를 입력하세요",
        "pw.too_short" => "최소 4자 이상 입력하세요",
        "pw.mismatch" => "비밀번호가 일치하지 않습니다",
        "pw.key_generated" => "마스터 키 생성 완료",
        "pw.save_fail" => "설정 저장 실패: {}",
        "pw.decrypt_ok" => "설정 복호화 성공",
        "pw.3fail_reset" => "3회 실패. [R]을 눌러 설정을 리셋하세요",
        "pw.wrong_pw" => "비밀번호 오류 ({}/3)",

        // === 온보딩 ===
        "onboard.save_ok" => "설정 저장 완료 → 대시보드",
        "onboard.save_fail" => "❌ 설정 저장 실패: {} — 디스크 용량/권한을 확인하세요",
        "onboard.llm_status_wait" => "검증 대기",
        "onboard.llm_status_testing" => "검증 중 (최대 5초)",
        "onboard.llm_status_fail_retry" => "[V]로 재시도",
        "onboard.tg_status_wait" => "검증 대기 (선택사항)",
        "onboard.tg_status_testing" => "검증 중 (최대 5초)",
        "onboard.tg_status_ok" => "Telegram Bot 확인됨",
        "onboard.tg_status_fail_retry" => "[V]로 재시도",

        // === 부트 ===
        "boot.init_msg" => "femtoClaw 시작",

        // === 피드 ===
        "feed.llm_verify_ok" => "LLM 검증 성공: {} — {}개 모델 발견",
        "feed.llm_verify_ok_simple" => "LLM 검증 성공: {}",
        "feed.llm_verify_fail" => "LLM 검증 실패: {}",
        "feed.tg_verify_ok" => "Telegram 검증 성공: {}",
        "feed.tg_verify_fail" => "Telegram 검증 실패: {}",

        // === 대시보드 ===
        "dash.agent_status" => "━━ 에이전트 상태 ━━",
        "dash.agent_name" => "에이전트: {}",
        "dash.model" => "모델: {} ({})",
        "dash.security" => "보안: Jailing=ON | ChaCha20=ON",
        "dash.llm_none" => "LLM 미설정",
        "dash.skill_header" => "━━ 스킬 목록 (TOML + Rhai) ━━",
        "dash.skill_builtin" => "[내장][{}] {} — {}",
        "dash.skill_core_fail" => "core 로드 실패: {}",
        "dash.skill_user" => "[사용자][{}] {} — {}",
        "dash.skill_user_fail" => "user 로드 실패: {}",
        "dash.timemachine_header" => "━━ 타임머신 (최근 10건) ━━",
        "dash.timemachine_cols" => "유형|상태|요약|시각",
        "dash.no_records" => "(기록 없음)",
        "dash.total_count" => "── 전체 {} 건 ──",
        "dash.db_query_fail" => "DB 조회 실패: {}",
        "dash.db_open_fail" => "DB 열기 실패: {}",
        "dash.agent_switch_header" => "━━ 에이전트 전환 ━━",
        "dash.no_agents" => "(등록된 에이전트 없음)",
        "dash.active" => "활성",
        "dash.inactive" => "비활성",
        "dash.agent_switched" => "→ 에이전트 #{}({})로 전환!",
        "dash.no_switch" => "(전환 가능한 다른 에이전트 없음)",
        "dash.agent_added" => "✅ 에이전트 #{} ({}) 추가 완료",
        "dash.agent_add_fail" => "❌ {}",

        // === CLI/Headless ===
        "cli.no_config" => "config.enc가 없습니다. TUI 모드에서 먼저 설정하세요",
        "cli.enter_pw" => "마스터 비밀번호: ",
        "cli.no_telegram" => "텔레그램 설정이 없거나 미검증 상태입니다",
        "cli.paired" => "페어링 성공: {} (chat_id: {})",
        "cli.chat_saved" => "chat_id 저장 완료 — 재시작 후에도 페어링 유지",
        "cli.chat_save_fail" => "페어링 chat_id 저장 실패: {}",
        "cli.msg_received" => "메시지 수신: {}",
        "cli.bot_shutdown" => "봇 종료됨",
        "cli.graceful_shutdown" => "Graceful Shutdown 완료",

        // === 도구 ===
        "tool.level.safe" => "안전",
        "tool.level.jail" => "Jail 검증",
        "tool.level.restricted" => "제한됨",
        "tool.file_read.name" => "파일 읽기",
        "tool.file_write.name" => "파일 쓰기",
        "tool.file_list.name" => "디렉토리 목록",
        "tool.sleep.name" => "대기",
        "tool.print.name" => "로그 출력",

        // === 텔레그램 봇 ===
        "bot.pair_prompt" => "🔐 페어링이 필요합니다. /pair <PIN>으로 인증하세요.",
        "bot.pair_success" => "✅ 페어링 완료! 이제 대화할 수 있습니다.",
        "bot.pair_fail" => "❌ PIN이 일치하지 않습니다. 다시 시도하세요.",
        "bot.help" => "📋 명령어:\n/pair <PIN> — 페어링\n/status — 상태\n/agent <N> — 전환",

        // === DB ===
        "db.type.user_msg" => "사용자 메시지",
        "db.type.agent_resp" => "에이전트 응답",
        "db.type.file_op" => "파일 작업",
        "db.type.config_change" => "설정 변경",
        "db.type.tool_call" => "도구 호출",
        "db.type.security_event" => "보안 이벤트",

        // === 검증 ===
        "val.timeout" => "타임아웃 (5초)",
        "val.connect_fail" => "연결 실패 — 서버가 실행 중인지 확인하세요",
        "val.bot_confirmed" => "봇 확인됨",
        "val.check_token" => "토큰이 올바른지 확인하세요",

        _ => return None,
    })
}
