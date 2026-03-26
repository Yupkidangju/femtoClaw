// femtoClaw — 설정(Config) 관리 모듈
// [v0.3.0] Step 1/8a: config.enc 파일의 직렬화/역직렬화 및 디스크 I/O.
// [v0.3.0] 멀티 에이전트 설정 모델 추가 (AgentConfig, 최대 3개)
//
// AppConfig 구조체를 JSON으로 직렬화한 후 crypto 모듈로 암호화하여 저장하고,
// 복호화 후 역직렬화하여 로드한다.
// spec.md 3.5절: 유효성 검증(HTTP 200 OK) 없이 API 키를 저장하는 것은 금지.
// 이 모듈은 저장/로드만 담당하며, 검증은 Step 2 (TUI Onboarding)에서 수행한다.

use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::{FemtoError, FemtoResult};
use crate::security::crypto;

/// [v0.1.0] LLM 공급자 프리셋 종류.
/// spec.md 3.1절: 2-Format, Multi-Preset 전략에 대응한다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum LlmPreset {
    /// OpenAI 호환 형식 공급자 (엔드포인트 URL만 다름)
    OpenAi,
    Gemini,
    Claude,
    XAi,
    OpenRouter,
    /// Ollama 형식 공급자 (로컬 LLM)
    Ollama,
    LmStudio,
    /// 사용자 정의 (임의 엔드포인트)
    Custom,
}

/// [v0.1.0] LLM 공급자 설정.
/// 프리셋 선택 시 endpoint가 자동 채워지며, Custom일 때만 사용자가 직접 입력한다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LlmProviderConfig {
    /// 선택된 프리셋 이름
    pub preset: LlmPreset,
    /// API 엔드포인트 URL (프리셋별 기본값 또는 사용자 입력값)
    pub endpoint: String,
    /// API 키 (검증 후 저장됨)
    pub api_key: String,
    /// 사용할 모델명 (예: "gpt-4", "gemini-2.5-pro")
    pub model: String,
    /// 검증 완료 여부 (HTTP 200 OK 확인 후 true)
    pub verified: bool,
}

/// [v0.1.0] 텔레그램 봇 설정.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TelegramConfig {
    /// 텔레그램 봇 토큰 (검증 후 저장됨)
    pub bot_token: String,
    /// 페어링된 채팅 ID (페어링 완료 후 저장됨)
    pub chat_id: Option<i64>,
    /// 검증 완료 여부
    pub verified: bool,
}

/// [v0.3.0] 개별 에이전트 설정.
/// 각 에이전트는 별도의 LLM 설정과 이름을 가진다.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AgentConfig {
    /// 에이전트 ID (1~3)
    pub id: u8,
    /// 에이전트 이름
    pub name: String,
    /// LLM 공급자 설정
    pub llm_provider: Option<LlmProviderConfig>,
    /// 활성 여부
    pub active: bool,
}

impl AgentConfig {
    /// 기본 에이전트 생성
    pub fn new(id: u8, name: &str) -> Self {
        Self {
            id,
            name: name.to_string(),
            llm_provider: None,
            active: true,
        }
    }
}

/// [v0.3.0] 앓 전체 설정 구조체.
/// config.enc에 암호화되어 저장되는 최상위 데이터 모델.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct AppConfig {
    /// [v0.1 호환] 단일 LLM 공급자 설정 (에이전트 #1의 바로가기)
    pub llm_provider: Option<LlmProviderConfig>,
    /// 텔레그램 봇 설정 (전체 공유)
    pub telegram: Option<TelegramConfig>,
    /// [v0.1 호환] 활성 에이전트 이름
    pub agent_name: String,
    /// [v0.3.0] 멀티 에이전트 목록 (최대 3개)
    #[serde(default)]
    pub agents: Vec<AgentConfig>,
    /// [v0.3.0] 현재 활성 에이전트 ID
    #[serde(default = "default_active_agent")]
    pub active_agent_id: u8,
}

fn default_active_agent() -> u8 {
    1
}

impl Default for AppConfig {
    fn default() -> Self {
        // [v0.3.0] 기본 에이전트 #1 자동 생성
        Self {
            llm_provider: None,
            telegram: None,
            agent_name: "Alpha".to_string(),
            agents: vec![AgentConfig::new(1, "Alpha")],
            active_agent_id: 1,
        }
    }
}

impl AppConfig {
    /// [v0.3.0] 활성 에이전트 설정 반환
    pub fn active_agent(&self) -> Option<&AgentConfig> {
        self.agents.iter().find(|a| a.id == self.active_agent_id)
    }

    /// [v0.3.0] 활성 에이전트 설정 가변 참조
    pub fn active_agent_mut(&mut self) -> Option<&mut AgentConfig> {
        let id = self.active_agent_id;
        self.agents.iter_mut().find(|a| a.id == id)
    }

    /// [v0.3.0] 에이전트 추가 (최대 3개 제한)
    pub fn add_agent(&mut self, name: &str) -> Result<u8, String> {
        if self.agents.len() >= 3 {
            return Err(msg!("err.max_agents").to_string());
        }
        let next_id = self.agents.iter().map(|a| a.id).max().unwrap_or(0) + 1;
        self.agents.push(AgentConfig::new(next_id, name));
        Ok(next_id)
    }

    /// [v0.3.0] 에이전트 전환
    pub fn switch_agent(&mut self, id: u8) -> Result<(), String> {
        if self.agents.iter().any(|a| a.id == id && a.active) {
            self.active_agent_id = id;
            Ok(())
        } else {
            Err(format!("Agent #{} not found or inactive", id))
        }
    }

    /// [v1.1.0] 에이전트 삭제 (기본 #1은 삭제 불가)
    pub fn remove_agent(&mut self, id: u8) -> Result<(), String> {
        if id == 1 {
            return Err("기본 에이전트 #1은 삭제할 수 없습니다.".to_string());
        }
        let original_len = self.agents.len();
        self.agents.retain(|a| a.id != id);
        if self.agents.len() == original_len {
            return Err(format!("Agent #{} not found", id));
        }
        // 삭제된 에이전트가 활성이면 #1로 전환
        if self.active_agent_id == id {
            self.active_agent_id = 1;
        }
        Ok(())
    }
}

/// [v0.1.0] AppConfig를 암호화하여 config.enc 파일로 저장한다.
///
/// 동작 순서:
/// 1. AppConfig → JSON 직렬화
/// 2. JSON 바이트 → ChaCha20Poly1305 암호화 (crypto::seal)
/// 3. 암호화된 바이트 → 파일 쓰기
pub fn save_config(config: &AppConfig, password: &[u8], path: &Path) -> FemtoResult<()> {
    // JSON 직렬화
    let json = serde_json::to_vec(config).map_err(FemtoError::Serialization)?;

    // 암호화
    let sealed = crypto::seal(password, &json)?;

    // 파일 쓰기
    fs::write(path, sealed).map_err(FemtoError::ConfigIo)?;

    Ok(())
}

/// [v0.1.0] config.enc 파일을 복호화하여 AppConfig로 로드한다.
///
/// 동작 순서:
/// 1. 파일 읽기 → 암호화된 바이트
/// 2. ChaCha20Poly1305 복호화 (crypto::unseal)
/// 3. JSON 바이트 → AppConfig 역직렬화
///
/// 패스워드가 틀리면 Decryption 에러를 반환한다.
pub fn load_config(password: &[u8], path: &Path) -> FemtoResult<AppConfig> {
    // 파일 읽기
    let sealed = fs::read(path).map_err(FemtoError::ConfigIo)?;

    // 복호화
    let json = crypto::unseal(password, &sealed)?;

    // JSON 역직렬화
    let config: AppConfig = serde_json::from_slice(&json).map_err(FemtoError::Serialization)?;

    Ok(config)
}

/// [v0.1.0] config.enc 파일이 존재하는지 확인한다.
/// 최초 실행(온보딩 필요) vs 재실행(비밀번호 입력만) 분기에 사용된다.
pub fn config_exists(path: &Path) -> bool {
    path.exists() && path.is_file()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;

    /// 설정 저장/로드 왕복 테스트:
    /// 저장한 설정을 로드하면 원본과 동일해야 한다.
    #[test]
    fn test_config_save_load_roundtrip() {
        let config = AppConfig {
            llm_provider: Some(LlmProviderConfig {
                preset: LlmPreset::Gemini,
                endpoint: "https://generativelanguage.googleapis.com/v1beta/openai".to_string(),
                api_key: "AIzaSy-test-key-1234567890".to_string(),
                model: "gemini-2.5-pro".to_string(),
                verified: true,
            }),
            telegram: Some(TelegramConfig {
                bot_token: "1234567890:ABCDEF-test-token".to_string(),
                chat_id: Some(123456789),
                verified: true,
            }),
            agent_name: "TestAgent".to_string(),
            agents: vec![AgentConfig::new(1, "TestAgent")],
            active_agent_id: 1,
        };

        let password = b"roundtrip-test-password";
        let temp_path = env::temp_dir().join("femtoclaw_test_config.enc");

        // 저장
        save_config(&config, password, &temp_path).expect("저장 성공해야 함");

        // 파일이 생성되었는지 확인
        assert!(temp_path.exists());

        // 로드
        let loaded = load_config(password, &temp_path).expect("로드 성공해야 함");
        assert_eq!(loaded, config);

        // 정리
        let _ = fs::remove_file(&temp_path);
    }

    /// 잘못된 비밀번호로 로드 시 에러 테스트
    #[test]
    fn test_load_with_wrong_password() {
        let config = AppConfig::default();
        let temp_path = env::temp_dir().join("femtoclaw_test_wrong_pw.enc");

        save_config(&config, b"correct", &temp_path).expect("저장 성공");

        let result = load_config(b"incorrect", &temp_path);
        assert!(result.is_err());

        let _ = fs::remove_file(&temp_path);
    }

    /// 기본 설정 직렬화 테스트:
    /// Default 설정이 올바르게 직렬화/역직렬화되는지 확인.
    #[test]
    fn test_default_config_serialization() {
        let config = AppConfig::default();
        let json = serde_json::to_string(&config).expect("직렬화 성공");
        let deserialized: AppConfig = serde_json::from_str(&json).expect("역직렬화 성공");
        assert_eq!(config, deserialized);
    }
}
