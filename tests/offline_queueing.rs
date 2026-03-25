use femtoclaw::core::chat_loop::ChatSession;
use femtoclaw::config::{LlmPreset, LlmProviderConfig};
use femtoclaw::core::persona::Persona;
use tempfile::tempdir;

#[test]
fn test_offline_queueing_persistence() {
    let dir = tempdir().unwrap();
    let workspace = dir.path();

    let llm_config = LlmProviderConfig {
        preset: LlmPreset::OpenAi,
        // 일부러 잘못된 URL을 넣어 LLM 호출을 강제 실패 처리
        endpoint: "http://localhost:9999/invalid_endpoint_to_fail".to_string(),
        api_key: "INVALID_KEY".to_string(),
        model: "test-model".to_string(),
        verified: false,
    };
    let persona = Persona::new_default("femto_test");

    // 첫 번째 세션: 메시지 전송 시도 -> 실패 -> 큐에 저장되어야 함
    {
        let mut session = ChatSession::new(&llm_config, &persona, workspace);
        let reply = session.handle_message("Are you there?");
        
        // 응답에 에러 표시가 포함되어야 함
        assert!(reply.contains("⚠️"), "Reply should contain warning symbol");
        
        // 큐에 1개의 메시지가 있어야 함
        assert_eq!(session.pending_count(), 1, "There should be 1 pending message");
    }

    // 세션이 drop됨. 디스크에 기록되었는지 확인
    let queue_file = workspace.join("pending_queue.json");
    assert!(queue_file.exists(), "pending_queue.json must be persisted to disk");

    let content = std::fs::read_to_string(&queue_file).unwrap();
    assert!(content.contains("Are you there?"), "Persisted queue must contain the user message");

    // 두 번째 세션: 새로운 세션 시작 시 파일에서 큐를 정상적으로 복구하는지 확인
    {
        // LLM은 여전히 실패하므로 drain 시 다시 큐로 돌아갈 것임
        let mut session = ChatSession::new(&llm_config, &persona, workspace);
        assert_eq!(session.pending_count(), 1, "Queue should be reloaded from disk");
        
        // drain을 시도해도 실패해서 큐에 남아야 함
        let reply2 = session.handle_message("Second failing message");
        assert!(reply2.contains("⚠️"));
        assert_eq!(session.pending_count(), 2, "Both messages should be queued");
    }

    // 파일 업데이트 확인
    let content2 = std::fs::read_to_string(&queue_file).unwrap();
    assert!(content2.contains("Second failing message"), "Second message must be saved");
}
