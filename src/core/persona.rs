// femtoClaw — 에이전트 페르소나 모듈
// [v0.6.0] agent.toml 파싱 → Persona 구조체
//
// 설계 원칙:
//   - OpenClaw의 AGENTS.md + SOUL.md + IDENTITY.md를 하나의 TOML로 통합
//   - serde로 직접 파싱, 구조화된 설정
//   - system prompt 조립 시 persona 정보를 주입

use serde::{Deserialize, Serialize};
use std::path::Path;

/// [v0.6.0] 에이전트 정체성
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Identity {
    /// 에이전트 이름 (예: "Alpha")
    pub name: String,
    /// 이모지 (예: "🐾")
    #[serde(default = "default_emoji")]
    pub emoji: String,
    /// 역할 설명 (예: "General-purpose AI assistant")
    #[serde(default = "default_role")]
    pub role: String,
    /// 기본 응답 언어
    #[serde(default = "default_language")]
    pub language: String,
}

fn default_emoji() -> String {
    "🐾".to_string()
}
fn default_role() -> String {
    "General-purpose AI assistant".to_string()
}
fn default_language() -> String {
    "en".to_string()
}

/// [v0.6.0] 에이전트 영혼 (성격, 톤, 경계)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Soul {
    /// 성격 특성 (예: "Friendly, precise, curious")
    #[serde(default = "default_personality")]
    pub personality: String,
    /// 커뮤니케이션 톤 (예: "Professional but warm")
    #[serde(default = "default_tone")]
    pub tone: String,
    /// 행동 경계 (절대 하지 않을 것)
    #[serde(default)]
    pub boundaries: Vec<String>,
}

fn default_personality() -> String {
    "Helpful, precise, and friendly".to_string()
}
fn default_tone() -> String {
    "Professional but approachable".to_string()
}

/// [v0.6.0] 운영 규칙
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Rules {
    /// 시작 시 수행할 행동
    #[serde(default)]
    pub startup: Vec<String>,
    /// 메시지 수신 시 규칙
    #[serde(default)]
    pub on_message: Vec<String>,
    /// 도구 연속 실패 최대 횟수
    #[serde(default = "default_max_retries")]
    pub max_tool_retries: u8,
    /// 컨텍스트 윈도우 토큰 제한
    #[serde(default = "default_context_window")]
    pub context_window_tokens: usize,
}

fn default_max_retries() -> u8 {
    3
}
fn default_context_window() -> usize {
    8192
}

/// [v0.6.0] 도구 설정
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolsConfig {
    /// 활성화된 도구 ID 목록
    #[serde(default = "default_tools")]
    pub enabled: Vec<String>,
}

fn default_tools() -> Vec<String> {
    vec![
        "file_read".into(),
        "file_write".into(),
        "file_list".into(),
        "print".into(),
        "sleep".into(),
    ]
}

/// [v0.6.0] 에이전트 페르소나 전체 (agent.toml 최상위)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Persona {
    pub identity: Identity,
    #[serde(default)]
    pub soul: Soul,
    #[serde(default)]
    pub rules: Rules,
    #[serde(default)]
    pub tools: ToolsConfig,
}

impl Default for Soul {
    fn default() -> Self {
        Self {
            personality: default_personality(),
            tone: default_tone(),
            boundaries: vec![
                "Never execute destructive commands".into(),
                "Always suggest alternatives when blocked".into(),
                "Ask for confirmation before file modifications".into(),
            ],
        }
    }
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            startup: vec![
                "Load MEMORY.md for long-term context".into(),
                "Load today's daily log for recent context".into(),
                "Greet user briefly".into(),
            ],
            on_message: vec![
                "Think step-by-step before using tools".into(),
                "Prefer reading files before writing".into(),
                "Log important decisions to MEMORY.md".into(),
            ],
            max_tool_retries: default_max_retries(),
            context_window_tokens: default_context_window(),
        }
    }
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            enabled: default_tools(),
        }
    }
}

impl Persona {
    /// [v0.6.0] agent.toml 파일에서 페르소나를 로드한다.
    /// 파일이 없으면 None 반환 (bootstrap 필요 신호).
    pub fn load(workspace: &Path) -> Option<Self> {
        let path = workspace.join("agent.toml");
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    /// [v0.6.0] 페르소나를 agent.toml로 저장한다.
    pub fn save(&self, workspace: &Path) -> Result<(), String> {
        let path = workspace.join("agent.toml");
        let content =
            toml::to_string_pretty(self).map_err(|e| format!("Serialization error: {}", e))?;
        std::fs::write(&path, content).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }

    /// [v0.6.0] 기본 페르소나 생성 (이름 지정)
    pub fn new_default(name: &str) -> Self {
        Self {
            identity: Identity {
                name: name.to_string(),
                emoji: default_emoji(),
                role: default_role(),
                language: default_language(),
            },
            soul: Soul::default(),
            rules: Rules::default(),
            tools: ToolsConfig::default(),
        }
    }

    /// [v0.6.0] system prompt에 주입할 persona 블록을 생성한다.
    pub fn to_system_block(&self) -> String {
        let mut block = String::with_capacity(1024);

        // 정체성
        block.push_str(&format!(
            "# Agent: {} {}\n\nRole: {}\nLanguage: {}\n\n",
            self.identity.name, self.identity.emoji, self.identity.role, self.identity.language
        ));

        // 성격
        block.push_str(&format!(
            "## Personality\n{}\nTone: {}\n\n",
            self.soul.personality, self.soul.tone
        ));

        // 경계
        if !self.soul.boundaries.is_empty() {
            block.push_str("## Boundaries\n");
            for b in &self.soul.boundaries {
                block.push_str(&format!("- {}\n", b));
            }
            block.push('\n');
        }

        // 규칙
        if !self.rules.on_message.is_empty() {
            block.push_str("## Rules\n");
            for r in &self.rules.on_message {
                block.push_str(&format!("- {}\n", r));
            }
            block.push('\n');
        }

        block
    }
}

/// [v0.6.0] 사용자 프로필 (user.toml)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserProfile {
    #[serde(default)]
    pub profile: UserInfo,
    #[serde(default)]
    pub preferences: UserPreferences,
    #[serde(default)]
    pub notes: UserNotes,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserInfo {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub nickname: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default = "default_timezone")]
    pub timezone: String,
}

fn default_timezone() -> String {
    "UTC".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UserPreferences {
    /// "brief" | "normal" | "detailed"
    #[serde(default = "default_verbosity")]
    pub verbosity: String,
    /// 파일 쓰기 전 확인 요청
    #[serde(default = "default_true")]
    pub confirm_writes: bool,
}

fn default_verbosity() -> String {
    "normal".to_string()
}
fn default_true() -> bool {
    true
}

impl Default for UserPreferences {
    fn default() -> Self {
        Self {
            verbosity: default_verbosity(),
            confirm_writes: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserNotes {
    /// 에이전트가 대화에서 학습한 내용
    #[serde(default)]
    pub learned: Vec<String>,
}

impl UserProfile {
    /// user.toml 로드
    pub fn load(workspace: &Path) -> Option<Self> {
        let path = workspace.join("user.toml");
        if !path.exists() {
            return None;
        }
        let content = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&content).ok()
    }

    /// user.toml 저장
    pub fn save(&self, workspace: &Path) -> Result<(), String> {
        let path = workspace.join("user.toml");
        let content =
            toml::to_string_pretty(self).map_err(|e| format!("Serialization error: {}", e))?;
        std::fs::write(&path, content).map_err(|e| format!("Write error: {}", e))?;
        Ok(())
    }

    /// 기본 프로필 생성
    pub fn new_default(name: &str, language: &str) -> Self {
        Self {
            profile: UserInfo {
                name: name.to_string(),
                language: language.to_string(),
                ..Default::default()
            },
            preferences: UserPreferences::default(),
            notes: UserNotes::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persona_default() {
        let p = Persona::new_default("Alpha");
        assert_eq!(p.identity.name, "Alpha");
        assert_eq!(p.identity.emoji, "🐾");
        assert_eq!(p.rules.max_tool_retries, 3);
        assert_eq!(p.rules.context_window_tokens, 8192);
        assert_eq!(p.tools.enabled.len(), 5);
    }

    #[test]
    fn test_persona_toml_roundtrip() {
        let p = Persona::new_default("Beta");
        let toml_str = toml::to_string_pretty(&p).unwrap();
        let parsed: Persona = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.identity.name, "Beta");
        assert_eq!(parsed.soul.boundaries.len(), p.soul.boundaries.len());
    }

    #[test]
    fn test_persona_system_block() {
        let p = Persona::new_default("Gamma");
        let block = p.to_system_block();
        assert!(block.contains("Gamma"));
        assert!(block.contains("🐾"));
        assert!(block.contains("Boundaries"));
        assert!(block.contains("destructive"));
    }

    #[test]
    fn test_user_profile_default() {
        let u = UserProfile::new_default("TestUser", "ko");
        assert_eq!(u.profile.name, "TestUser");
        assert_eq!(u.profile.language, "ko");
        assert!(u.preferences.confirm_writes);
    }

    #[test]
    fn test_user_profile_roundtrip() {
        let u = UserProfile::new_default("User", "en");
        let toml_str = toml::to_string_pretty(&u).unwrap();
        let parsed: UserProfile = toml::from_str(&toml_str).unwrap();
        assert_eq!(parsed.profile.name, "User");
    }
}
