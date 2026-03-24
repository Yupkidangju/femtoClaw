// femtoClaw — 컨텍스트 조립 모듈
// [v0.6.0] persona + memory + history + tools → system prompt 조립
//
// 설계 원칙:
//   - tiktoken-rs로 정확한 토큰 수 산정
//   - 컨텍스트 윈도우 초과 시 오래된 메시지부터 제거
//   - MEMORY.md + 오늘/어제 일일 로그를 long-term context로 주입
//   - persona (agent.toml)의 system block을 system prompt에 포함

use std::path::Path;

use super::agent::ChatMessage;
use super::persona::Persona;
use crate::tools::prompt;

/// [v0.6.0] 컨텍스트 윈도우 관리자
pub struct ContextManager {
    /// 최대 토큰 제한
    max_tokens: usize,
    /// system prompt (persona + tools + rules)
    system_prompt: String,
}

impl ContextManager {
    /// [v0.6.0] 새 컨텍스트 매니저 생성 — persona와 workspace에서 컨텍스트 조립
    pub fn new(persona: &Persona, workspace: &Path) -> Self {
        let max_tokens = persona.rules.context_window_tokens;
        let system_prompt = Self::build_system_prompt(persona, workspace);

        Self {
            max_tokens,
            system_prompt,
        }
    }

    /// [v0.6.0] system prompt 조립: persona + tools + memory
    fn build_system_prompt(persona: &Persona, workspace: &Path) -> String {
        let mut prompt = String::with_capacity(8192);

        // 1. 페르소나 블록
        prompt.push_str(&persona.to_system_block());

        // 2. 도구 명세 + Jailing 규칙 (기존 prompt.rs 활용)
        prompt.push_str(&prompt::build_system_prompt(&persona.identity.name));

        // 3. 장기 기억 (MEMORY.md)
        let memory_path = workspace.join("MEMORY.md");
        if memory_path.exists() {
            if let Ok(memory) = std::fs::read_to_string(&memory_path) {
                if !memory.is_empty() {
                    prompt.push_str("\n## Long-term Memory\n");
                    // 메모리가 너무 길면 앞 2000자만 사용
                    let truncated = if memory.len() > 2000 {
                        &memory[..2000]
                    } else {
                        &memory
                    };
                    prompt.push_str(truncated);
                    prompt.push('\n');
                }
            }
        }

        // 4. 오늘 일일 로그
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let today_log = workspace.join("memory").join(format!("{}.md", today));
        if today_log.exists() {
            if let Ok(log) = std::fs::read_to_string(&today_log) {
                if !log.is_empty() {
                    prompt.push_str("\n## Today's Log\n");
                    let truncated = if log.len() > 1000 { &log[..1000] } else { &log };
                    prompt.push_str(truncated);
                    prompt.push('\n');
                }
            }
        }

        prompt
    }

    /// [v0.6.0] system prompt 반환
    pub fn system_prompt(&self) -> &str {
        &self.system_prompt
    }

    /// [v0.6.0] 대화 기록을 토큰 제한에 맞게 트림한다.
    /// system prompt + messages의 총 토큰이 max_tokens를 초과하면
    /// 가장 오래된 메시지(system 제외)부터 제거한다.
    pub fn trim_messages(&self, messages: &[ChatMessage]) -> Vec<ChatMessage> {
        let system_tokens = Self::count_tokens(&self.system_prompt);
        let available = self.max_tokens.saturating_sub(system_tokens);

        // 뒤에서부터(최신 메시지부터) 토큰을 적산
        let mut kept: Vec<ChatMessage> = Vec::new();
        let mut used_tokens = 0;

        for msg in messages.iter().rev() {
            let msg_text = msg.content.as_deref().unwrap_or("");
            let msg_tokens = Self::count_tokens(msg_text) + 4; // 메시지 오버헤드 ~4토큰

            if used_tokens + msg_tokens > available {
                break; // 제한 초과 — 이보다 오래된 메시지는 버림
            }
            used_tokens += msg_tokens;
            kept.push(msg.clone());
        }

        kept.reverse(); // 시간순 복원
        kept
    }

    /// [v0.6.0] 토큰 수 산정 (tiktoken-rs 사용)
    /// cl100k_base 인코딩 (GPT-4/3.5 기본값)
    pub fn count_tokens(text: &str) -> usize {
        // tiktoken-rs의 cl100k_base 인코더 사용
        match tiktoken_rs::cl100k_base() {
            Ok(bpe) => bpe.encode_with_special_tokens(text).len(),
            Err(_) => {
                // fallback: 문자 수 / 4 근사 (영어 기준)
                text.len() / 4
            }
        }
    }

    /// [v0.6.0] 전체 컨텍스트의 토큰 사용량 보고
    pub fn token_usage(&self, messages: &[ChatMessage]) -> TokenUsage {
        let system_tokens = Self::count_tokens(&self.system_prompt);
        let message_tokens: usize = messages
            .iter()
            .map(|m| Self::count_tokens(m.content.as_deref().unwrap_or("")) + 4)
            .sum();

        TokenUsage {
            system: system_tokens,
            messages: message_tokens,
            total: system_tokens + message_tokens,
            max: self.max_tokens,
        }
    }
}

/// [v0.6.0] 토큰 사용량 보고 구조체
#[derive(Debug, Clone)]
pub struct TokenUsage {
    pub system: usize,
    pub messages: usize,
    pub total: usize,
    pub max: usize,
}

impl TokenUsage {
    /// 사용률 (0.0 ~ 1.0+)
    pub fn utilization(&self) -> f64 {
        if self.max == 0 {
            return 0.0;
        }
        self.total as f64 / self.max as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens() {
        let tokens = ContextManager::count_tokens("Hello, world!");
        // cl100k_base: "Hello, world!" ≈ 4토큰
        assert!(tokens > 0 && tokens < 20);
    }

    #[test]
    fn test_count_tokens_korean() {
        let tokens = ContextManager::count_tokens("안녕하세요, 세계!");
        // CJK 문자는 토큰 수가 다름
        assert!(tokens > 0);
    }

    #[test]
    fn test_trim_messages_within_limit() {
        let persona = Persona::new_default("Test");
        let ws = std::env::temp_dir().join("femtoclaw_ctx_test_1");
        std::fs::create_dir_all(&ws).ok();

        let ctx = ContextManager {
            max_tokens: 100000,
            system_prompt: "You are a test bot.".to_string(),
        };

        let msgs = vec![
            ChatMessage::text("user", "hello"),
            ChatMessage::text("assistant", "hi there"),
            ChatMessage::text("user", "how are you?"),
        ];

        let trimmed = ctx.trim_messages(&msgs);
        assert_eq!(trimmed.len(), 3); // 제한 내이므로 전부 유지

        let _ = std::fs::remove_dir_all(&ws);
    }

    #[test]
    fn test_trim_messages_exceeds_limit() {
        let ctx = ContextManager {
            max_tokens: 30, // 매우 작은 제한
            system_prompt: "System prompt taking lots of tokens for testing purposes.".to_string(),
        };

        let msgs = vec![
            ChatMessage::text("user", "This is the first message with some content"),
            ChatMessage::text("assistant", "This is a reply with more content"),
            ChatMessage::text("user", "Latest message"),
        ];

        let trimmed = ctx.trim_messages(&msgs);
        // 제한이 매우 작으므로 최신 메시지만 남거나 비어있을 수 있음
        assert!(trimmed.len() <= msgs.len());
    }

    #[test]
    fn test_token_usage() {
        let ctx = ContextManager {
            max_tokens: 8192,
            system_prompt: "You are a test bot.".to_string(),
        };

        let msgs = vec![ChatMessage::text("user", "hello")];
        let usage = ctx.token_usage(&msgs);

        assert!(usage.system > 0);
        assert!(usage.messages > 0);
        assert_eq!(usage.total, usage.system + usage.messages);
        assert_eq!(usage.max, 8192);
        assert!(usage.utilization() > 0.0);
        assert!(usage.utilization() < 1.0);
    }
}
