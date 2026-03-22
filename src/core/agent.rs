// femtoClaw — LLM 에이전트 대화 클라이언트
// [v0.1.0] Step 4: OpenAI 호환 /chat/completions API 호출.
//
// 2-Format 전략:
//   - OpenAI 호환: Authorization: Bearer {key} + /chat/completions
//   - Ollama: 인증 불필요 + /api/chat
//
// 에이전트는 단일 대화 루프:
//   사용자 메시지 → LLM API 호출 → 응답 반환 → DB 기록

use crate::config::LlmPreset;

/// LLM 에이전트 설정
#[derive(Debug, Clone)]
pub struct AgentConfig {
    pub preset: LlmPreset,
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
}

/// 대화 메시지 (OpenAI 형식)
#[derive(Debug, Clone, serde::Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// LLM 응답
#[derive(Debug, Clone)]
pub struct AgentResponse {
    pub content: String,
    pub model: String,
    pub tokens_used: Option<u32>,
}

/// [v0.1.0] LLM API에 대화를 전송하고 응답을 받는다.
/// OpenAI 호환 /chat/completions 형식과 Ollama /api/chat 형식을 자동 분기.
pub async fn chat(config: &AgentConfig, messages: &[ChatMessage]) -> Result<AgentResponse, String> {
    let client = reqwest::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(5))
        .timeout(std::time::Duration::from_secs(120)) // LLM은 응답이 느릴 수 있음
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    match config.preset {
        LlmPreset::Ollama | LlmPreset::LmStudio => chat_ollama(&client, config, messages).await,
        _ => chat_openai_compatible(&client, config, messages).await,
    }
}

/// OpenAI 호환 API (/chat/completions) 호출
async fn chat_openai_compatible(
    client: &reqwest::Client,
    config: &AgentConfig,
    messages: &[ChatMessage],
) -> Result<AgentResponse, String> {
    let url = format!("{}/chat/completions", config.endpoint);

    let body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "max_tokens": 4096,
    });

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", config.api_key))
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "LLM 응답 타임아웃 (120초)".to_string()
            } else if e.is_connect() {
                "LLM 서버 연결 실패".to_string()
            } else {
                format!("LLM 요청 실패: {}", e)
            }
        })?;

    if !response.status().is_success() {
        return Err(format!("LLM API 오류: HTTP {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("응답 파싱 실패: {}", e))?;

    // OpenAI 응답 형식: choices[0].message.content
    let content = json["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("[응답 없음]")
        .to_string();

    let model = json["model"].as_str().unwrap_or(&config.model).to_string();

    let tokens = json["usage"]["total_tokens"].as_u64().map(|t| t as u32);

    Ok(AgentResponse {
        content,
        model,
        tokens_used: tokens,
    })
}

/// Ollama API (/api/chat) 호출
async fn chat_ollama(
    client: &reqwest::Client,
    config: &AgentConfig,
    messages: &[ChatMessage],
) -> Result<AgentResponse, String> {
    let url = format!("{}/api/chat", config.endpoint);

    let body = serde_json::json!({
        "model": config.model,
        "messages": messages,
        "stream": false,
    });

    let response = client
        .post(&url)
        .header("Content-Type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            if e.is_timeout() {
                "Ollama 응답 타임아웃 (120초)".to_string()
            } else if e.is_connect() {
                "Ollama 서버 연결 실패 — ollama serve 실행 확인".to_string()
            } else {
                format!("Ollama 요청 실패: {}", e)
            }
        })?;

    if !response.status().is_success() {
        return Err(format!("Ollama API 오류: HTTP {}", response.status()));
    }

    let json: serde_json::Value = response
        .json()
        .await
        .map_err(|e| format!("응답 파싱 실패: {}", e))?;

    // Ollama 응답 형식: message.content
    let content = json["message"]["content"]
        .as_str()
        .unwrap_or("[응답 없음]")
        .to_string();

    let model = json["model"].as_str().unwrap_or(&config.model).to_string();

    // Ollama는 토큰 사용량을 eval_count로 제공
    let tokens = json["eval_count"].as_u64().map(|t| t as u32);

    Ok(AgentResponse {
        content,
        model,
        tokens_used: tokens,
    })
}
