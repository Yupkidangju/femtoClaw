// femtoClaw — 스킬 파일 로더 (TOML + Rhai)
// [v0.2.0] Step 5/6b: skills/core/ 및 skills/user/ 디렉토리에서
// .toml 정적 스킬과 .rhai 동적 스킬 파일을 일괄 로드한다.
//
// 스킬 파일 형식 (TOML):
// ```toml
// [skill]
// name = "파일 읽기"
// description = "지정된 파일의 내용을 읽어 반환합니다"
// version = "1.0"
//
// [prompt]
// template = "다음 파일을 읽고 내용을 요약하세요: {file_path}"
// system = "당신은 파일 분석 전문가입니다."
//
// [actions]
// allowed = ["file_read", "file_list"]
// ```

use std::path::{Path, PathBuf};

/// 스킬이 수행할 수 있는 동작
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum SkillAction {
    /// 파일 읽기
    #[serde(rename = "file_read")]
    FileRead,
    /// 파일 쓰기
    #[serde(rename = "file_write")]
    FileWrite,
    /// 파일 목록 조회
    #[serde(rename = "file_list")]
    FileList,
    /// 웹 검색
    #[serde(rename = "web_search")]
    WebSearch,
    /// 명령어 실행 (블랙리스트 가드 적용)
    #[serde(rename = "command_exec")]
    CommandExec,
    /// 대화만 (외부 동작 없음)
    #[serde(rename = "chat_only")]
    ChatOnly,
}

/// 프롬프트 템플릿
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PromptTemplate {
    /// 프롬프트 템플릿 (변수: {file_path}, {query} 등)
    pub template: String,
    /// 시스템 프롬프트 (선택)
    #[serde(default)]
    pub system: Option<String>,
}

/// 스킬 정의
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SkillDef {
    pub name: String,
    pub description: String,
    #[serde(default = "default_version")]
    pub version: String,
}

fn default_version() -> String {
    "1.0".to_string()
}

/// 허용 동작 목록
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ActionsDef {
    pub allowed: Vec<SkillAction>,
}

/// TOML 파일 최상위 구조
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct SkillToml {
    skill: SkillDef,
    prompt: PromptTemplate,
    actions: ActionsDef,
}

/// [v0.2.0] 스킬 유형
#[derive(Debug, Clone, PartialEq)]
pub enum SkillType {
    /// TOML 정적 스킬 (프롬프트 템플릿 기반)
    Static,
    /// Rhai 동적 스킬 (스크립트 실행)
    Dynamic,
}

/// 로드된 스킬 (파일 경로 포함)
#[derive(Debug, Clone)]
pub struct Skill {
    pub name: String,
    pub description: String,
    pub version: String,
    pub prompt_template: String,
    pub system_prompt: Option<String>,
    pub allowed_actions: Vec<SkillAction>,
    /// 스킬 파일 경로 (core/ 또는 user/)
    pub source_path: PathBuf,
    /// 내장 스킬 여부
    pub is_builtin: bool,
    /// [v0.2.0] 스킬 유형 (Static = TOML, Dynamic = Rhai)
    pub skill_type: SkillType,
}

/// [v0.2.0] 지정 디렉토리에서 .toml 및 .rhai 스킬 파일을 모두 로드한다.
pub fn load_skills_from_dir(dir: &Path, is_builtin: bool) -> Result<Vec<Skill>, String> {
    if !dir.exists() {
        return Ok(Vec::new());
    }

    let mut skills = Vec::new();

    let entries = std::fs::read_dir(dir)
        .map_err(|e| format!("스킬 디렉토리 읽기 실패 {}: {}", dir.display(), e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("항목 읽기 실패: {}", e))?;
        let path = entry.path();
        let ext = path.extension().and_then(|e| e.to_str());

        match ext {
            Some("toml") => {
                // TOML 정적 스킬 로드
                match load_skill_file(&path, is_builtin) {
                    Ok(skill) => skills.push(skill),
                    Err(e) => eprintln!("[경고] TOML 스킬 로드 실패 {}: {}", path.display(), e),
                }
            }
            Some("rhai") => {
                // [v0.2.0] Rhai 동적 스킬 로드
                match load_rhai_skill(&path, is_builtin) {
                    Ok(skill) => skills.push(skill),
                    Err(e) => eprintln!("[경고] Rhai 스킬 로드 실패 {}: {}", path.display(), e),
                }
            }
            _ => continue,
        }
    }

    Ok(skills)
}

/// 단일 TOML 스킬 파일 로드
fn load_skill_file(path: &Path, is_builtin: bool) -> Result<Skill, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("파일 읽기 실패: {}", e))?;

    let toml_data: SkillToml =
        toml::from_str(&content).map_err(|e| format!("TOML 파싱 실패: {}", e))?;

    Ok(Skill {
        name: toml_data.skill.name,
        description: toml_data.skill.description,
        version: toml_data.skill.version,
        prompt_template: toml_data.prompt.template,
        system_prompt: toml_data.prompt.system,
        allowed_actions: toml_data.actions.allowed,
        source_path: path.to_path_buf(),
        is_builtin,
        skill_type: SkillType::Static,
    })
}

/// [v0.2.0] Rhai 스킬 파일 로드 — 파일 첫 줄 주석에서 메타데이터 추출
/// 형식: // @name: 스킬이름 | @desc: 설명
fn load_rhai_skill(path: &Path, is_builtin: bool) -> Result<Skill, String> {
    let content = std::fs::read_to_string(path).map_err(|e| format!("파일 읽기 실패: {}", e))?;

    // 첫 번째 주석 줄에서 메타데이터 추출
    let mut name = path
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("unknown")
        .to_string();
    let mut description = "Rhai 동적 스킬".to_string();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with("//") {
            break;
        }
        let comment = trimmed.trim_start_matches("//").trim();
        if let Some(n) = comment.strip_prefix("@name:") {
            name = n.trim().to_string();
        } else if let Some(d) = comment.strip_prefix("@desc:") {
            description = d.trim().to_string();
        }
    }

    Ok(Skill {
        name,
        description,
        version: "1.0".to_string(),
        prompt_template: String::new(), // Rhai는 스크립트 자체가 동작
        system_prompt: None,
        allowed_actions: vec![], // Rhai는 호스트 함수로 동작 제한
        source_path: path.to_path_buf(),
        is_builtin,
        skill_type: SkillType::Dynamic,
    })
}

/// [v0.1.0] 새 스킬을 TOML 파일로 저장한다.
/// skills/user/ 디렉토리에 저장.
pub fn save_skill(skill: &Skill, user_dir: &Path) -> Result<PathBuf, String> {
    std::fs::create_dir_all(user_dir).map_err(|e| format!("스킬 디렉토리 생성 실패: {}", e))?;

    let toml_data = SkillToml {
        skill: SkillDef {
            name: skill.name.clone(),
            description: skill.description.clone(),
            version: skill.version.clone(),
        },
        prompt: PromptTemplate {
            template: skill.prompt_template.clone(),
            system: skill.system_prompt.clone(),
        },
        actions: ActionsDef {
            allowed: skill.allowed_actions.clone(),
        },
    };

    let content =
        toml::to_string_pretty(&toml_data).map_err(|e| format!("TOML 직렬화 실패: {}", e))?;

    // 파일명: 스킬 이름을 snake_case로 변환
    let filename = skill
        .name
        .to_lowercase()
        .replace(' ', "_")
        .replace(|c: char| !c.is_alphanumeric() && c != '_', "");
    let path = user_dir.join(format!("{}.toml", filename));

    std::fs::write(&path, content).map_err(|e| format!("스킬 저장 실패: {}", e))?;

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_skill_dir() -> PathBuf {
        let dir = std::env::temp_dir()
            .join("femtoclaw_skill_test")
            .join(format!(
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_load_skill_toml() {
        let dir = temp_skill_dir();

        // 테스트 스킬 파일 생성
        let toml_content = r#"
[skill]
name = "파일 읽기"
description = "지정된 파일의 내용을 읽어 반환합니다"
version = "1.0"

[prompt]
template = "다음 파일을 읽으세요: {file_path}"
system = "파일 분석 전문가"

[actions]
allowed = ["file_read", "file_list"]
"#;
        fs::write(dir.join("read_file.toml"), toml_content).unwrap();

        let skills = load_skills_from_dir(&dir, true).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "파일 읽기");
        assert_eq!(skills[0].allowed_actions.len(), 2);
        assert!(skills[0].is_builtin);
        assert_eq!(skills[0].system_prompt.as_deref(), Some("파일 분석 전문가"));

        cleanup(&dir);
    }

    #[test]
    fn test_save_and_load_skill() {
        let dir = temp_skill_dir();

        let skill = Skill {
            name: "웹 검색".to_string(),
            description: "웹에서 정보를 검색합니다".to_string(),
            version: "1.0".to_string(),
            prompt_template: "{query}에 대해 검색하세요".to_string(),
            system_prompt: None,
            allowed_actions: vec![SkillAction::WebSearch, SkillAction::ChatOnly],
            source_path: PathBuf::new(),
            is_builtin: false,
            skill_type: SkillType::Static,
        };

        // 저장
        let path = save_skill(&skill, &dir).unwrap();
        assert!(path.exists());
        assert_eq!(path.file_name().unwrap().to_str().unwrap(), "웹_검색.toml");

        // 재로드
        let skills = load_skills_from_dir(&dir, false).unwrap();
        assert_eq!(skills.len(), 1);
        assert_eq!(skills[0].name, "웹 검색");
        assert!(!skills[0].is_builtin);

        cleanup(&dir);
    }

    #[test]
    fn test_malformed_toml_skipped() {
        let dir = temp_skill_dir();

        // 정상 파일
        fs::write(
            dir.join("good.toml"),
            r#"
[skill]
name = "Good"
description = "Works"
[prompt]
template = "test"
[actions]
allowed = ["chat_only"]
"#,
        )
        .unwrap();

        // 손상된 파일 (걸러져야 함)
        fs::write(dir.join("bad.toml"), "this is not valid toml {{{{").unwrap();

        // TXT 파일 (무시되어야 함)
        fs::write(dir.join("readme.txt"), "not a skill").unwrap();

        let skills = load_skills_from_dir(&dir, true).unwrap();
        assert_eq!(skills.len(), 1); // good.toml만 로드
        assert_eq!(skills[0].name, "Good");

        cleanup(&dir);
    }

    #[test]
    fn test_empty_dir() {
        let dir = temp_skill_dir();
        let skills = load_skills_from_dir(&dir, true).unwrap();
        assert!(skills.is_empty());
        cleanup(&dir);
    }

    #[test]
    fn test_nonexistent_dir() {
        let skills = load_skills_from_dir(Path::new("/nonexistent/path"), true).unwrap();
        assert!(skills.is_empty());
    }
}
