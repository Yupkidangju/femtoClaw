// femtoClaw — 채팅 루프 모듈
// [v0.6.0] 에이전트 대화의 핵심 런타임
//
// 설계 원칙:
//   - 단일 handle_message() 함수 — TUI, 텔레그램 모두 이것만 호출
//   - 게이트웨이/메시지 버스 없이 직접 호출하는 단순 동기 구조
//   - Function Calling: LLM → tool_calls → executor → 결과 피드백 → 재호출
//   - 대화 기록 자동 관리 (메모리 + 세션 트랜스크립트)

use std::path::{Path, PathBuf};

use super::agent::{AgentConfig, AgentResponse, ChatMessage};
use super::context::ContextManager;
use super::persona::Persona;
use super::tool_protocol;
use crate::config::LlmProviderConfig;
use crate::tools::executor::ToolExecutor;

/// [v0.6.0] 채팅 세션 상태
pub struct ChatSession {
    /// LLM 설정
    agent_config: AgentConfig,
    /// 컨텍스트 관리자 (토큰 카운터 + system prompt)
    context: ContextManager,
    /// 대화 기록
    history: Vec<ChatMessage>,
    /// 도구 실행기
    executor: ToolExecutor,
    /// workspace 경로
    workspace: PathBuf,
    /// OpenAI tools 파라미터 (사전 빌드)
    tool_definitions: Vec<super::agent::ToolDefinition>,
}

impl ChatSession {
    /// [v0.6.0] 새 채팅 세션 생성
    pub fn new(llm_config: &LlmProviderConfig, persona: &Persona, workspace: &Path) -> Self {
        let agent_config = AgentConfig {
            preset: llm_config.preset.clone(),
            endpoint: llm_config.endpoint.clone(),
            api_key: llm_config.api_key.clone(),
            model: llm_config.model.clone(),
        };

        let context = ContextManager::new(persona, workspace);
        let executor = ToolExecutor::new(workspace.to_path_buf());
        let tool_definitions = tool_protocol::build_tool_definitions();

        Self {
            agent_config,
            context,
            history: Vec::new(),
            executor,
            workspace: workspace.to_path_buf(),
            tool_definitions,
        }
    }

    /// [v0.6.0] 메시지 처리 — TUI와 텔레그램 모두 이 함수 하나를 호출
    ///
    /// 흐름:
    /// 1. 사용자 메시지를 history에 추가
    /// 2. 컨텍스트 트림 → system prompt + trimmed history 조립
    /// 3. LLM API 호출 (tools 파라미터 포함)
    /// 4. tool_calls 있으면: 실행 → 결과 피드백 → 재호출 (최대 5회)
    /// 5. 최종 텍스트 응답 반환
    /// 6. 일일 로그, 세션 트랜스크립트에 기록
    pub fn handle_message(&mut self, user_message: &str) -> String {
        // 1. 사용자 메시지 추가
        self.history.push(ChatMessage::text("user", user_message));

        // 2. LLM 호출 (blocking — tokio::runtime 사용)
        let response = self.call_llm_with_tools();

        // 3. 응답을 history에 추가
        let reply_text = match response {
            Ok(text) => text,
            Err(e) => format!("⚠️ {}", e),
        };

        self.history
            .push(ChatMessage::text("assistant", &reply_text));

        // 4. 일일 로그에 기록
        self.append_daily_log(user_message, &reply_text);

        reply_text
    }

    /// [v0.6.0] LLM 호출 + tool_call 연쇄 처리
    fn call_llm_with_tools(&mut self) -> Result<String, String> {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| format!("Tokio runtime error: {}", e))?;

        rt.block_on(async {
            let mut rounds = 0;

            loop {
                // 컨텍스트 트림
                let trimmed = self.context.trim_messages(&self.history);

                // system prompt + trimmed history 조립
                let mut messages = vec![ChatMessage::text("system", self.context.system_prompt())];
                messages.extend(trimmed);

                // LLM API 호출
                let response = super::agent::chat_with_tools(
                    &self.agent_config,
                    &messages,
                    &self.tool_definitions,
                )
                .await?;

                // 텍스트 응답이면 즉시 반환
                if !response.has_tool_calls() {
                    return Ok(response
                        .content
                        .unwrap_or_else(|| "[No response]".to_string()));
                }

                // tool_calls 처리
                rounds += 1;
                if rounds > tool_protocol::MAX_TOOL_ROUNDS {
                    return Ok(
                        "⚠️ Tool call limit reached. Please try a simpler request.".to_string()
                    );
                }

                // tool_calls를 assistant 메시지로 기록
                self.history.push(ChatMessage::assistant_tool_calls(
                    response.tool_calls.clone(),
                ));

                // 각 tool_call 실행 → tool role 메시지 추가
                for tc in &response.tool_calls {
                    let result = tool_protocol::execute_tool_call(&mut self.executor, tc);
                    let result_text = tool_protocol::format_tool_result(&result);

                    self.history.push(ChatMessage::tool_result(
                        &tc.id,
                        &tc.function.name,
                        &result_text,
                    ));
                }

                // 루프: 다음 LLM 호출에서 tool 결과를 참조하여 응답 생성
            }
        })
    }

    /// [v0.6.0] 일일 로그에 대화 기록 추가
    fn append_daily_log(&self, user_msg: &str, assistant_msg: &str) {
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        let log_dir = self.workspace.join("memory");
        let log_path = log_dir.join(format!("{}.md", today));

        let entry = format!(
            "\n### {} — Conversation\n- **User:** {}\n- **Agent:** {}\n",
            time,
            user_msg.chars().take(200).collect::<String>(),
            assistant_msg.chars().take(200).collect::<String>(),
        );

        // 로그 디렉토리 + 파일 자동 생성
        let _ = std::fs::create_dir_all(&log_dir);
        if !log_path.exists() {
            let header = format!("# Daily Log — {}\n", today);
            let _ = std::fs::write(&log_path, header);
        }

        // append
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(&log_path) {
            let _ = file.write_all(entry.as_bytes());
        }
    }

    /// [v0.6.0] 대화 기록 수 반환
    pub fn message_count(&self) -> usize {
        self.history.len()
    }

    /// [v0.6.0] 토큰 사용량 보고
    pub fn token_usage(&self) -> super::context::TokenUsage {
        self.context.token_usage(&self.history)
    }

    /// [v0.6.0] 대화 기록 초기화
    pub fn clear_history(&mut self) {
        self.history.clear();
    }

    /// [v0.6.0] 읽기 전용 대화 기록 접근
    pub fn history(&self) -> &[ChatMessage] {
        &self.history
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{LlmPreset, LlmProviderConfig};

    fn test_llm_config() -> LlmProviderConfig {
        LlmProviderConfig {
            preset: LlmPreset::Ollama,
            endpoint: "http://localhost:11434".to_string(),
            api_key: String::new(),
            model: "test-model".to_string(),
            verified: true,
        }
    }

    #[test]
    fn test_session_creation() {
        let ws = std::env::temp_dir().join("femtoclaw_chatloop_test");
        std::fs::create_dir_all(&ws).ok();

        let config = test_llm_config();
        let persona = Persona::new_default("TestBot");
        let session = ChatSession::new(&config, &persona, &ws);

        assert_eq!(session.message_count(), 0);
        assert!(session.history().is_empty());

        let usage = session.token_usage();
        assert!(usage.system > 0);
        assert_eq!(usage.messages, 0);

        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn test_session_clear_history() {
        let ws = std::env::temp_dir().join("femtoclaw_chatloop_test2");
        std::fs::create_dir_all(&ws).ok();

        let config = test_llm_config();
        let persona = Persona::new_default("TestBot");
        let mut session = ChatSession::new(&config, &persona, &ws);

        // 수동으로 history 추가
        session.history.push(ChatMessage::text("user", "hello"));
        session.history.push(ChatMessage::text("assistant", "hi"));
        assert_eq!(session.message_count(), 2);

        session.clear_history();
        assert_eq!(session.message_count(), 0);

        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn test_daily_log_creation() {
        let ws = std::env::temp_dir().join("femtoclaw_daily_test");
        std::fs::create_dir_all(ws.join("memory")).ok();

        let config = test_llm_config();
        let persona = Persona::new_default("TestBot");
        let session = ChatSession::new(&config, &persona, &ws);

        session.append_daily_log("hello", "hi there");

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let log_path = ws.join("memory").join(format!("{}.md", today));
        assert!(log_path.exists());

        let content = std::fs::read_to_string(&log_path).unwrap();
        assert!(content.contains("hello"));
        assert!(content.contains("hi there"));

        let _ = std::fs::remove_dir_all(&ws);
    }
}
