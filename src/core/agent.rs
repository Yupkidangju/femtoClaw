// femtoClaw — LLM 에이전트 대화 클라이언트
// [v0.1.0] Step 4: OpenAI 호환 /chat/completions API 호출.
// [v0.6.0] Function Calling 지원 추가 — tool_calls 응답 파싱
//
// 2-Format 전략:
//   - OpenAI 호환: Authorization: Bearer {key} + /chat/completions
//   - Ollama: 인증 불필요 + /api/chat
//
// [v0.6.0] 변경점:
//   - ChatMessage에 tool_call_id, tool_calls 필드 추가
//   - AgentResponse에 tool_calls 파싱 결과 포함
//   - chat_with_tools() 함수 추가 — tools 파라미터 포함 요청

use crate::config::LlmPreset;

/// LLM 에이전트 설정
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub preset: LlmPreset,
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
}

/// [v0.6.0] 대화 메시지 (OpenAI 형식 + Function Calling 지원)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ChatMessage {
    pub role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<String>,
    /// tool role 메시지에서 사용: 어떤 tool_call에 대한 응답인지
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    /// assistant 메시지에서 LLM이 반환한 tool_calls
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    /// function call의 이름 (tool role에서 사용)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
}

impl ChatMessage {
    /// 단순 텍스트 메시지 생성 (이전 호환)
    pub fn text(role: &str, content: &str) -> Self {
        Self {
            role: role.to_string(),
            content: Some(content.to_string()),
            tool_call_id: None,
            tool_calls: None,
            name: None,
        }
    }

    /// tool 실행 결과 메시지 생성
    pub fn tool_result(tool_call_id: &str, name: &str, result: &str) -> Self {
        Self {
            role: "tool".to_string(),
            content: Some(result.to_string()),
            tool_call_id: Some(tool_call_id.to_string()),
            tool_calls: None,
            name: Some(name.to_string()),
        }
    }

    /// assistant의 tool_calls 메시지 생성 (LLM 응답 재구성용)
    pub fn assistant_tool_calls(tool_calls: Vec<ToolCall>) -> Self {
        Self {
            role: "assistant".to_string(),
            content: None,
            tool_call_id: None,
            tool_calls: Some(tool_calls),
            name: None,
        }
    }
}

/// [v0.6.0] OpenAI Function Calling — tool_call 응답 구조체
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

/// [v0.6.0] function call 상세
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String, // JSON 문자열
}

/// LLM 응답
#[derive(Debug, Clone)]
pub struct AgentResponse {
    /// 텍스트 응답 (tool_call이 아닌 경우)
    pub content: Option<String>,
    pub model: String,
    pub tokens_used: Option<u32>,
    /// [v0.6.0] LLM이 반환한 tool_calls (있으면 도구 실행 필요)
    pub tool_calls: Vec<ToolCall>,
}

impl AgentResponse {
    /// tool_call이 있는지 확인
    pub fn has_tool_calls(&self) -> bool {
        !self.tool_calls.is_empty()
    }
}

/// [v0.6.0] OpenAI tools 파라미터용 도구 정의 (JSON Schema 형식)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: serde_json::Value,
}

/// [v0.6.0] Function Calling 포함 LLM API 호출
pub async fn chat_with_tools(
    config: &AgentConfig,
    messages: &[ChatMessage],
    tools: &[ToolDefinition],
) -> Result<AgentResponse, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client creation failed: {}", e))?;

    match config.preset {
        LlmPreset::Ollama | LlmPreset::LmStudio => {
            // Ollama도 최신 버전은 tools 지원
            chat_openai_with_tools(&client, config, messages, tools, false).await
        }
        _ => chat_openai_with_tools(&client, config, messages, tools, true).await,
    }
}

/// [v0.6.0] OpenAI 호환 API 호출 (tools 파라미터 포함)
async fn chat_openai_with_tools(
    client: &reqwest::Client,
    config: &AgentConfig,
    messages: &[ChatMessage],
    tools: &[ToolDefinition],
    use_auth: bool,
) -> Result<AgentResponse, String> {
    // 엔드포인트 결정
    let url = if config.preset == LlmPreset::Ollama || config.preset == LlmPreset::LmStudio {
        format!("{}/api/chat", config.endpoint)
    } else {
        format!("{}/chat/completions", config.endpoint)
    };

    // 요청 본문 구성
    let mut body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "max_tokens": 4096,
    });

    // tools가 있으면 추가
    if !tools.is_empty() {
        body["tools"] = serde_json::to_value(tools)
            .map_err(|e| format!("Tools serialization failed: {}", e))?;
    }

    // Ollama 전용: stream 비활성화
    if config.preset == LlmPreset::Ollama || config.preset == LlmPreset::LmStudio {
        body["stream"] = serde_json::Value::Bool(false);
    }

    // HTTP 요청
    let mut req = client.post(&url).header("Content-Type", "application/json");

    if use_auth {
        req = req.header("Authorization", format!("Bearer {}", config.api_key));
    }

    let response = req.json(&body).send().await.map_err(|e| {
        if e.is_timeout() {
            "LLM response timeout (120s)".to_string()
        } else if e.is_connect() {
            "LLM server connection failed".to_string()
        } else {
            format!("LLM request failed: {}", e)
        }
    })?;

    if !response.status().is_success() {
        return Err(format!("LLM API error: HTTP {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("Response parsing failed: {}", e))?;

    // 응답 파싱 — OpenAI/Ollama 공통
    let message = if json.get("choices").is_some() {
        // OpenAI 호환 형식
        &json["choices"][0]["message"]
    } else {
        // Ollama 형식
        &json["message"]
    };

    // 텍스트 응답
    let content = message["content"].as_str().map(|s| s.to_string());

    // tool_calls 파싱
    let tool_calls = if let Some(tc_array) = message.get("tool_calls").and_then(|v| v.as_array()) {
        tc_array
            .iter()
            .filter_map(|tc| serde_json::from_value::<ToolCall>(tc.clone()).ok())
            .collect()
    } else {
        Vec::new()
    };

    let model = json
        .get("model")
        .and_then(|v| v.as_str())
        .unwrap_or(&config.model)
        .to_string();

    let tokens = json
        .get("usage")
        .and_then(|u| u.get("total_tokens"))
        .and_then(|t| t.as_u64())
        .or_else(|| json.get("eval_count").and_then(|v| v.as_u64()))
        .map(|t| t as u32);

    Ok(AgentResponse {
        content,
        model,
        tokens_used: tokens,
        tool_calls,
    })
}

/// [v0.1.0 호환] 도구 없는 단순 대화 (이전 인터페이스 유지)
pub async fn chat(config: &AgentConfig, messages: &[ChatMessage]) -> Result<AgentResponse, String> {
    chat_with_tools(config, messages, &[]).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chat_message_text() {
        let msg = ChatMessage::text("user", "hello");
        assert_eq!(msg.role, "user");
        assert_eq!(msg.content.unwrap(), "hello");
        assert!(msg.tool_call_id.is_none());
    }

    #[test]
    fn test_chat_message_tool_result() {
        let msg = ChatMessage::tool_result("call_123", "file_read", "file contents here");
        assert_eq!(msg.role, "tool");
        assert_eq!(msg.tool_call_id.unwrap(), "call_123");
        assert_eq!(msg.name.unwrap(), "file_read");
    }

    #[test]
    fn test_agent_response_has_tool_calls() {
        let empty = AgentResponse {
            content: Some("hello".into()),
            model: "test".into(),
            tokens_used: None,
            tool_calls: vec![],
        };
        assert!(!empty.has_tool_calls());

        let with_tools = AgentResponse {
            content: None,
            model: "test".into(),
            tokens_used: None,
            tool_calls: vec![ToolCall {
                id: "call_1".into(),
                call_type: "function".into(),
                function: FunctionCall {
                    name: "file_read".into(),
                    arguments: r#"{"path":"test.txt"}"#.into(),
                },
            }],
        };
        assert!(with_tools.has_tool_calls());
    }

    #[test]
    fn test_tool_call_serialization() {
        let tc = ToolCall {
            id: "call_abc".into(),
            call_type: "function".into(),
            function: FunctionCall {
                name: "file_read".into(),
                arguments: r#"{"path":"data/report.txt"}"#.into(),
            },
        };
        let json = serde_json::to_string(&tc).unwrap();
        assert!(json.contains("call_abc"));
        assert!(json.contains("file_read"));

        // 역직렬화
        let parsed: ToolCall = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.id, "call_abc");
    }
}
