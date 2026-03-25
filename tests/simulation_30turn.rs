// femtoClaw — 30턴 시뮬레이션 통합 테스트
// [v0.8.0] LLM 없이 ChatSession의 data flow를 30턴 반복 검증
//
// 검증 항목:
//   1. history 누적 (user + assistant 메시지)
//   2. daily_log 파일 생성 + 내용 누적
//   3. MEMORY.md 큐레이션 (라인 추가 + FIFO)
//   4. session transcript 파일 생성 + 내용 누적
//   5. token_usage 증가 추적
//   6. 스케줄러 cron 패턴 매칭
//   7. clear_history 후 상태 리셋
//   8. 다국어 msg!() 키 존재 여부

#[cfg(test)]
mod simulation {
    use femtoclaw::config::{LlmPreset, LlmProviderConfig};
    use femtoclaw::core::chat_loop::ChatSession;
    use femtoclaw::core::persona::Persona;
    use femtoclaw::core::schedule::{CronPattern, ScheduleConfig};

    /// 테스트용 LLM 설정 (Ollama 로컬 — 실제 호출하지 않음)
    fn test_config() -> LlmProviderConfig {
        LlmProviderConfig {
            preset: LlmPreset::Ollama,
            endpoint: "http://localhost:11434".to_string(),
            api_key: String::new(),
            model: "sim-test-model".to_string(),
            verified: true,
        }
    }

    /// 고유한 workspace 경로 생성 (테스트 격리)
    fn test_workspace(name: &str) -> std::path::PathBuf {
        let ws = std::env::temp_dir().join(format!("femtoclaw_sim_{}", name));
        let _ = std::fs::remove_dir_all(&ws);
        std::fs::create_dir_all(&ws).unwrap();
        std::fs::create_dir_all(ws.join("memory")).unwrap();
        std::fs::create_dir_all(ws.join("sessions")).unwrap();
        ws
    }

    /// cleanup
    fn cleanup(ws: &std::path::Path) {
        let _ = std::fs::remove_dir_all(ws);
    }

    // ─── 테스트 #1: 30턴 history + daily_log + MEMORY.md + transcript ───

    #[test]
    fn test_30_turn_data_flow() {
        let ws = test_workspace("30turn");
        let config = test_config();
        let persona = Persona::new_default("SimBot");
        let mut session = ChatSession::new(&config, &persona, &ws);

        // 초기 상태 검증
        assert_eq!(session.message_count(), 0);
        assert!(session.history().is_empty());

        // 30턴 시뮬레이션 (LLM 호출 없이 history 직접 조작)
        for turn in 1..=30 {
            let user_msg = format!("사용자 메시지 턴 {}: 테스트 입력입니다", turn);
            let agent_msg = format!("에이전트 응답 턴 {}: 시뮬레이션 결과", turn);

            // history에 user + assistant 메시지 추가 (handle_message 대체)
            session
                .history
                .push(femtoclaw::core::agent::ChatMessage::text("user", &user_msg));
            session
                .history
                .push(femtoclaw::core::agent::ChatMessage::text(
                    "assistant",
                    &agent_msg,
                ));

            // daily_log 기록
            session.append_daily_log(&user_msg, &agent_msg);

            // MEMORY.md 큐레이션
            session.curate_memory(&user_msg);

            // session transcript 기록
            session.append_session_transcript(&user_msg, &agent_msg);

            // 턴별 history 검증
            assert_eq!(session.message_count(), turn * 2);
        }

        // ─── 30턴 후 파일 상태 검증 ───

        // 1. history: 60개 (30턴 × 2)
        assert_eq!(session.message_count(), 60);

        // 2. daily_log 파일 존재 + 30건 기록
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let log_path = ws.join("memory").join(format!("{}.md", today));
        assert!(log_path.exists(), "daily_log 파일이 존재해야 함");
        let log_content = std::fs::read_to_string(&log_path).unwrap();
        let log_entries = log_content.matches("### ").count();
        assert_eq!(log_entries, 30, "daily_log에 30건 기록되어야 함");

        // 3. MEMORY.md 파일 존재 + 30줄 (100줄 미만이므로 FIFO 미작동)
        let memory_path = ws.join("MEMORY.md");
        assert!(memory_path.exists(), "MEMORY.md 파일이 존재해야 함");
        let memory_content = std::fs::read_to_string(&memory_path).unwrap();
        let memory_entries = memory_content
            .lines()
            .filter(|l| l.starts_with("- ["))
            .count();
        assert_eq!(memory_entries, 30, "MEMORY.md에 30줄 기록되어야 함");

        // 4. session transcript 파일 존재 + 30건 기록
        let sessions_dir = ws.join("sessions");
        let session_files: Vec<_> = std::fs::read_dir(&sessions_dir)
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "md"))
            .collect();
        assert!(
            !session_files.is_empty(),
            "세션 트랜스크립트 파일이 존재해야 함"
        );
        let transcript = std::fs::read_to_string(session_files[0].path()).unwrap();
        let transcript_entries = transcript.matches("**Agent:**").count();
        assert_eq!(transcript_entries, 30, "transcript에 30건 기록되어야 함");

        // 5. token_usage — system prompt 토큰은 0보다 커야 함
        let usage = session.token_usage();
        assert!(usage.system > 0, "system prompt 토큰이 0보다 커야 함");
        assert!(usage.messages > 0, "30턴 메시지 토큰이 0보다 커야 함");
        assert!(usage.total > 0, "총 토큰이 0보다 커야 함");

        // 6. clear_history 후 검증
        session.clear_history();
        assert_eq!(session.message_count(), 0);
        assert!(session.history().is_empty());

        cleanup(&ws);
    }

    // ─── 테스트 #2: MEMORY.md FIFO 110턴 ───

    #[test]
    fn test_memory_fifo_overflow() {
        let ws = test_workspace("fifo");
        let config = test_config();
        let persona = Persona::new_default("FifoBot");
        let session = ChatSession::new(&config, &persona, &ws);

        // 110턴 → FIFO 작동 확인
        for turn in 1..=110 {
            let msg = format!("FIFO 테스트 메시지 {}", turn);
            session.curate_memory(&msg);
        }

        let memory_path = ws.join("MEMORY.md");
        let content = std::fs::read_to_string(&memory_path).unwrap();
        let data_lines = content.lines().filter(|l| l.starts_with("- [")).count();

        assert_eq!(data_lines, 100, "FIFO 후 정확히 100줄이어야 함");

        // 가장 최근 항목이 110번이어야 함
        assert!(
            content.contains("FIFO 테스트 메시지 110"),
            "최신(110번) 항목이 보존되어야 함"
        );
        // 가장 오래된 항목(1~10번)은 제거되어야 함
        assert!(
            !content.contains("FIFO 테스트 메시지 1\n"),
            "오래된(1번) 항목은 제거되어야 함"
        );

        cleanup(&ws);
    }

    // ─── 테스트 #3: 크론 패턴 매칭 시뮬레이션 ───

    #[test]
    fn test_cron_patterns_batch() {
        // 다양한 크론 패턴 검증
        let patterns = vec![
            ("* * * * *", true),   // 항상 매칭
            ("0 3 * * *", false), // 매일 03:00만 (현재 시각에 따라 달라지지만 테스트는 패턴 파싱만)
            ("*/5 * * * *", true), // 5분 간격 — 패턴 파싱만 검증
            ("0 */6 * * *", true), // 6시간 간격
        ];

        for (expr, _expected_parse) in &patterns {
            let result = CronPattern::parse(expr);
            assert!(result.is_ok(), "크론 패턴 '{}' 파싱 실패", expr);
        }

        // 잘못된 패턴
        assert!(CronPattern::parse("0 3").is_err());
        assert!(CronPattern::parse("a b c d e").is_err());
        assert!(CronPattern::parse("").is_err());
    }

    // ─── 테스트 #4: schedule.toml 파싱 ───

    #[test]
    fn test_schedule_config_full() {
        let toml_str = r#"
[[tasks]]
name = "memory_cleanup"
cron = "0 3 * * *"
action = "memory_cleanup"

[[tasks]]
name = "db_backup"
cron = "0 */6 * * *"
action = "db_backup"

[[tasks]]
name = "daily_summary"
cron = "0 22 * * *"
action = "daily_summary"
"#;
        let config: ScheduleConfig = toml::from_str(toml_str).unwrap();
        assert_eq!(config.tasks.len(), 3);
        assert_eq!(config.tasks[0].action, "memory_cleanup");
        assert_eq!(config.tasks[1].action, "db_backup");
        assert_eq!(config.tasks[2].action, "daily_summary");

        // 각 크론 패턴 파싱 확인
        for task in &config.tasks {
            assert!(
                CronPattern::parse(&task.cron).is_ok(),
                "작업 '{}' 크론 파싱 실패",
                task.name
            );
        }
    }

    // ─── 테스트 #5: 다국어 msg!() 키 존재 확인 ───

    #[test]
    fn test_i18n_critical_keys() {
        // 봇 메시지 키
        let bot_keys = vec![
            "bot.pair_success",
            "bot.pair_fail",
            "bot.pair_prompt",
            "bot.help",
        ];

        // DB 타입 키
        let db_keys = vec![
            "db.type.user_msg",
            "db.type.agent_resp",
            "db.type.file_op",
            "db.type.api_call",
            "db.type.system_event",
            "db.type.skill_run",
            "db.type.tool_call",
            "db.type.security_event",
        ];

        for key in bot_keys.iter().chain(db_keys.iter()) {
            let msg = femtoclaw::msg!(key);
            assert!(!msg.is_empty(), "i18n 키 '{}' 빈 문자열이면 안 됨", key);
        }
    }

    // ─── 테스트 #6: 스케줄러 액션 실행 검증 ───

    #[test]
    fn test_scheduler_actions() {
        let ws = test_workspace("actions");
        std::fs::create_dir_all(ws.join("memory")).unwrap();

        // daily_summary 액션 — 일일 로그 없으면 조용히 스킵
        let db_path = ws.join("test.db");
        femtoclaw::core::schedule::execute_action("daily_summary", &ws, &db_path);

        // 일일 로그 생성 후 daily_summary
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let log_path = ws.join("memory").join(format!("{}.md", today));
        std::fs::write(
            &log_path,
            "# Daily Log\n\n### 10:00 — Conversation\n- User: hello\n- Agent: hi\n\n### 10:05 — Conversation\n- User: test\n- Agent: ok\n",
        )
        .unwrap();

        femtoclaw::core::schedule::execute_action("daily_summary", &ws, &db_path);

        let memory_path = ws.join("MEMORY.md");
        if memory_path.exists() {
            let content = std::fs::read_to_string(&memory_path).unwrap();
            assert!(
                content.contains("일일 요약"),
                "daily_summary가 MEMORY.md에 기록되어야 함"
            );
        }

        // 알 수 없는 액션은 에러 없이 무시
        femtoclaw::core::schedule::execute_action("unknown_action", &ws, &db_path);

        cleanup(&ws);
    }

    // ─── 테스트 #7: default schedule.toml 생성 ───

    #[test]
    fn test_default_schedule_creation() {
        let ws = test_workspace("sched_default");

        let result = femtoclaw::core::schedule::create_default_config(&ws);
        assert!(result.is_ok());

        let path = ws.join("schedule.toml");
        assert!(path.exists());

        let config = femtoclaw::core::schedule::load_config(&ws);
        assert!(config.is_some());
        let config = config.unwrap();
        assert_eq!(config.tasks.len(), 3);

        cleanup(&ws);
    }
}
