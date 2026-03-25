// femtoClaw — 채팅 루프 모듈
// [v0.6.0] 에이전트 대화의 핵심 런타임
// [v0.7.0] ChatWorker: background thread로 LLM 호출 분리 (TUI 비동기)
//
// 설계 원칙:
//   - 단일 handle_message() 함수 — TUI, 텔레그램 모두 이것만 호출
//   - 게이트웨이/메시지 버스 없이 직접 호출하는 단순 동기 구조
//   - Function Calling: LLM → tool_calls → executor → 결과 피드백 → 재호출
//   - 대화 기록 자동 관리 (메모리 + 세션 트랜스크립트)
//   - [v0.7.0] ChatWorker: TUI용 비동기 래퍼 (background thread + mpsc 채널)

use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use super::agent::{AgentConfig, ChatMessage};
use super::context::ContextManager;
use super::persona::Persona;
use super::tool_protocol;
use crate::config::LlmProviderConfig;
use crate::tools::executor::ToolExecutor;

/// [v0.7.0] 비동기 채팅 응답 유형 (TUI tick()에서 수신)
#[derive(Debug, Clone)]
pub enum ChatEvent {
    /// LLM 호출 시작됨 — "생각 중..." 표시용
    Thinking,
    /// 도구 실행 중 — 어떤 도구가 사용되었는지 알림
    ToolUsed(String),
    /// 최종 텍스트 응답
    Reply(String),
    /// 에러 발생
    Error(String),
}

/// [v0.7.0] TUI용 비동기 채팅 워커
/// ChatSession을 background thread로 이동하여 UI blocking을 방지한다.
///
/// 사용법:
///   let worker = ChatWorker::spawn(llm_config, persona, workspace);
///   worker.send("hello");       // 비동기 전송
///   // tick()에서:
///   while let Some(event) = worker.try_recv() { ... }
pub struct ChatWorker {
    /// 사용자 메시지 전송 채널
    request_tx: mpsc::Sender<String>,
    /// LLM 응답 수신 채널
    response_rx: mpsc::Receiver<ChatEvent>,
    /// 토큰 사용량 조회용 공유 상태
    token_state: std::sync::Arc<std::sync::Mutex<TokenState>>,
    /// 현재 LLM 호출 중인지 여부
    busy: std::sync::Arc<std::sync::atomic::AtomicBool>,
}

/// [v0.7.0] 공유 토큰 상태 (thread-safe)
#[derive(Debug, Clone, Default)]
pub struct TokenState {
    pub system: usize,
    pub messages: usize,
    pub total: usize,
    pub max: usize,
    pub message_count: usize,
}

impl TokenState {
    pub fn utilization(&self) -> f64 {
        if self.max == 0 {
            return 0.0;
        }
        self.total as f64 / self.max as f64
    }
}

impl ChatWorker {
    /// [v0.7.0] 새 ChatWorker 생성 + background thread 시작
    /// [v0.8.0] db_path: DB ActionLog 기록용 경로 (None이면 기록 생략)
    pub fn spawn(
        llm_config: &LlmProviderConfig,
        persona: &Persona,
        workspace: &Path,
        db_path: Option<PathBuf>,
    ) -> Self {
        let (request_tx, request_rx) = mpsc::channel::<String>();
        let (response_tx, response_rx) = mpsc::channel::<ChatEvent>();
        let token_state = std::sync::Arc::new(std::sync::Mutex::new(TokenState::default()));
        let busy = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

        // ChatSession은 background thread가 소유
        let mut session = ChatSession::new(llm_config, persona, workspace);
        // [v0.8.0] DB 기록 활성화
        if let Some(path) = db_path {
            session.set_db_path(path);
        }

        let ts = token_state.clone();
        let busy_flag = busy.clone();

        // background thread 시작
        std::thread::spawn(move || {
            // 초기 토큰 상태 업데이트
            Self::update_token_state(&session, &ts);

            while let Ok(user_msg) = request_rx.recv() {
                // "생각 중..." 알림
                busy_flag.store(true, std::sync::atomic::Ordering::Relaxed);
                let _ = response_tx.send(ChatEvent::Thinking);

                // 메시지 처리 (blocking — 이 thread에서는 OK)
                let reply = session.handle_message(&user_msg);

                // 토큰 상태 업데이트
                Self::update_token_state(&session, &ts);

                // 응답 전송
                let _ = response_tx.send(ChatEvent::Reply(reply));
                busy_flag.store(false, std::sync::atomic::Ordering::Relaxed);
            }
        });

        Self {
            request_tx,
            response_rx,
            token_state,
            busy,
        }
    }

    /// [v0.7.0] 사용자 메시지 비동기 전송
    pub fn send(&self, message: &str) -> bool {
        self.request_tx.send(message.to_string()).is_ok()
    }

    /// [v0.7.0] 응답 비동기 수신 (non-blocking)
    pub fn try_recv(&self) -> Option<ChatEvent> {
        self.response_rx.try_recv().ok()
    }

    /// [v0.7.0] 현재 LLM 호출 중인지 확인
    pub fn is_busy(&self) -> bool {
        self.busy.load(std::sync::atomic::Ordering::Relaxed)
    }

    /// [v0.7.0] 토큰 사용량 조회 (thread-safe)
    pub fn token_state(&self) -> TokenState {
        self.token_state
            .lock()
            .map(|s| s.clone())
            .unwrap_or_default()
    }

    /// [v0.7.0] 세션의 토큰 상태를 공유 상태에 반영
    fn update_token_state(
        session: &ChatSession,
        state: &std::sync::Arc<std::sync::Mutex<TokenState>>,
    ) {
        let usage = session.token_usage();
        if let Ok(mut s) = state.lock() {
            s.system = usage.system;
            s.messages = usage.messages;
            s.total = usage.total;
            s.max = usage.max;
            s.message_count = session.message_count();
        }
    }
}

/// [v0.6.0] 채팅 세션 상태
pub struct ChatSession {
    /// LLM 설정
    agent_config: AgentConfig,
    /// 컨텍스트 관리자 (토큰 카운터 + system prompt)
    context: ContextManager,
    /// 대화 기록
    pub history: Vec<ChatMessage>,
    /// 도구 실행기
    executor: ToolExecutor,
    /// workspace 경로
    workspace: PathBuf,
    /// OpenAI tools 파라미터 (사전 빌드)
    tool_definitions: Vec<super::agent::ToolDefinition>,
    /// [v0.7.0] 세션 트랜스크립트 파일 경로
    session_path: PathBuf,
    /// [v0.8.0] DB 파일 경로 (ActionLog 기록용, None이면 기록 생략)
    db_path: Option<PathBuf>,
    /// [v0.8.0] 오프라인 큐잉 — LLM API 실패 시 대기열
    pending_queue: Vec<String>,
}

/// [v0.7.0] MEMORY.md 최대 라인 수 (FIFO 정리 기준)
const MEMORY_MAX_LINES: usize = 100;

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

        // [v0.7.0] 세션 트랜스크립트 파일 초기화
        let sessions_dir = workspace.join("sessions");
        let _ = std::fs::create_dir_all(&sessions_dir);
        let session_name = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let session_path = sessions_dir.join(format!("{}.md", session_name));
        let header = format!(
            "# Session Transcript — {}\n\n> Model: {}\n\n",
            chrono::Local::now().format("%Y-%m-%d %H:%M:%S"),
            llm_config.model,
        );
        let _ = std::fs::write(&session_path, header);

        Self {
            agent_config,
            context,
            history: Vec::new(),
            executor,
            workspace: workspace.to_path_buf(),
            tool_definitions,
            session_path,
            db_path: None,
            pending_queue: Vec::new(),
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
        // 0. [v0.8.0] 오프라인 큐 드레인 — 이전 실패 메시지가 있으면 먼저 재시도
        self.drain_pending_queue();

        // 1. 사용자 메시지 추가
        self.history.push(ChatMessage::text("user", user_message));

        // 2. LLM 호출 (blocking — tokio::runtime 사용)
        let response = self.call_llm_with_tools();

        // 3. 응답을 history에 추가 / 실패 시 큐잉
        let reply_text = match response {
            Ok(text) => text,
            Err(e) => {
                // [v0.8.0] 오프라인 큐잉 — 사용자 메시지를 대기열에 저장
                self.pending_queue.push(user_message.to_string());
                // history에서 실패한 user 메시지는 제거 (다음 호출 시 재시도)
                self.history.pop();
                format!("⚠️ {} (큐 {}건 대기 중)", e, self.pending_queue.len())
            }
        };

        self.history
            .push(ChatMessage::text("assistant", &reply_text));

        // 4. 일일 로그에 기록
        self.append_daily_log(user_message, &reply_text);

        // 5. [v0.7.0] MEMORY.md 큐레이션
        self.curate_memory(user_message);

        // 6. [v0.7.0] 세션 트랜스크립트에 추가
        self.append_session_transcript(user_message, &reply_text);

        // 7. [v0.8.0] DB ActionLog 기록
        self.record_to_db(user_message, &reply_text);

        reply_text
    }

    /// [v0.8.0] DB 파일 경로 설정 (ActionLog 기록 활성화)
    pub fn set_db_path(&mut self, path: PathBuf) {
        self.db_path = Some(path);
    }

    /// [v0.8.0] DB에 UserMessage + AgentResponse 기록
    fn record_to_db(&self, user_msg: &str, agent_msg: &str) {
        let db_path = match &self.db_path {
            Some(p) => p,
            None => return,
        };

        let db = match crate::db::store::FemtoDb::open(db_path) {
            Ok(db) => db,
            Err(_) => return,
        };

        // 사용자 메시지 기록
        let user_summary: String = user_msg.chars().take(80).collect();
        let _ = db.insert_action(
            &crate::db::store::ActionType::UserMessage,
            &user_summary,
            user_msg,
        );

        // 에이전트 응답 기록
        let agent_summary: String = agent_msg.chars().take(80).collect();
        let _ = db.insert_action(
            &crate::db::store::ActionType::AgentResponse,
            &agent_summary,
            agent_msg,
        );
    }

    /// [v0.8.0] 오프라인 큐 드레인 — 이전 실패 메시지를 순서대로 재전송
    /// LLM이 정상 응답하면 해당 메시지를 큐에서 제거하고 history에 추가.
    /// 재실패하면 큐에 남겨두고 중단.
    fn drain_pending_queue(&mut self) {
        if self.pending_queue.is_empty() {
            return;
        }

        // 큐 복사 후 비우기 (실패 시 다시 넣음)
        let queued: Vec<String> = self.pending_queue.drain(..).collect();
        let mut still_pending = Vec::new();
        let mut drained = false;

        for msg in queued {
            self.history.push(ChatMessage::text("user", &msg));

            match self.call_llm_with_tools() {
                Ok(reply) => {
                    self.history.push(ChatMessage::text("assistant", &reply));
                    self.append_daily_log(&msg, &reply);
                    self.curate_memory(&msg);
                    self.append_session_transcript(&msg, &reply);
                    self.record_to_db(&msg, &reply);
                    drained = true;
                }
                Err(_) => {
                    // 재실패 — history에서 제거하고 큐에 복원
                    self.history.pop();
                    still_pending.push(msg);
                    // 나머지도 큐에 남겨둠
                    break;
                }
            }
        }

        // 실패한 것 + 남은 것 큐에 복원
        if !still_pending.is_empty() {
            self.pending_queue = still_pending;
            // 아직 처리하지 못한 나머지도 복원
            // (이미 drain 됐으므로 추가로 남은 것은 없음)
        }

        if drained {
            eprintln!(
                "[큐] {} 건 재전송 완료, {} 건 대기 중",
                if self.pending_queue.is_empty() {
                    "전체"
                } else {
                    "일부"
                },
                self.pending_queue.len()
            );
        }
    }

    /// [v0.8.0] 대기 중인 큐 크기 반환
    pub fn pending_count(&self) -> usize {
        self.pending_queue.len()
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
    pub fn append_daily_log(&self, user_msg: &str, assistant_msg: &str) {
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

    /// [v0.7.0] MEMORY.md 자동 큐레이션
    /// 매 대화 후 메모리 파일에 요약 라인을 추가한다.
    /// 100줄 초과 시 오래된 항목을 자동 제거 (FIFO).
    pub fn curate_memory(&self, user_msg: &str) {
        let memory_path = self.workspace.join("MEMORY.md");
        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M").to_string();
        let summary: String = user_msg.chars().take(50).collect();
        let entry = format!("- [{}] {}", timestamp, summary);

        // 기존 내용 읽기
        let mut lines: Vec<String> = if memory_path.exists() {
            std::fs::read_to_string(&memory_path)
                .unwrap_or_default()
                .lines()
                .map(|l| l.to_string())
                .collect()
        } else {
            vec!["# MEMORY.md — 대화 요약 기록".to_string(), String::new()]
        };

        // 새 항목 추가
        lines.push(entry);

        // FIFO: 헤더(2줄) + 데이터. 데이터만 MEMORY_MAX_LINES으로 제한
        let header_count = lines.iter().take_while(|l| !l.starts_with("- [")).count();
        let data_lines: Vec<String> = lines[header_count..].to_vec();
        if data_lines.len() > MEMORY_MAX_LINES {
            let trim = data_lines.len() - MEMORY_MAX_LINES;
            let trimmed: Vec<String> = lines[..header_count]
                .iter()
                .chain(data_lines[trim..].iter())
                .cloned()
                .collect();
            let _ = std::fs::write(&memory_path, trimmed.join("\n") + "\n");
        } else {
            let _ = std::fs::write(&memory_path, lines.join("\n") + "\n");
        }
    }

    /// [v0.7.0] 세션 트랜스크립트에 대화 내용 추가
    pub fn append_session_transcript(&self, user_msg: &str, assistant_msg: &str) {
        let time = chrono::Local::now().format("%H:%M:%S").to_string();
        let entry = format!(
            "---\n\n**[{}] User:**\n{}\n\n**Agent:**\n{}\n\n",
            time, user_msg, assistant_msg
        );

        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .append(true)
            .open(&self.session_path)
        {
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
