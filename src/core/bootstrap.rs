// femtoClaw — Bootstrap 모듈
// [v0.6.0] 첫 실행 감지 → BOOTSTRAP.md 의식 → agent.toml/user.toml/MEMORY.md 생성
//
// 설계 원칙:
//   - workspace에 agent.toml이 없으면 "첫 실행"으로 판단
//   - BOOTSTRAP.md를 시드로 생성하고, 기본 파일들을 세팅
//   - TUI 모드: 온보딩 완료 후 자동 실행
//   - Headless 모드: 기본값으로 즉시 생성
//   - BOOTSTRAP.md는 완료 후 삭제 (1회성)

use std::fs;
use std::path::Path;

use super::persona::{Persona, UserProfile};

/// [v0.6.0] Bootstrap 상태
#[derive(Debug, Clone, PartialEq)]
pub enum BootstrapState {
    /// 이미 초기화됨 (agent.toml 존재)
    Ready,
    /// 첫 실행 — Bootstrap 필요
    NeedsBootstrap,
}

/// [v0.6.0] workspace의 Bootstrap 상태를 확인한다.
pub fn check_state(workspace: &Path) -> BootstrapState {
    let agent_toml = workspace.join("agent.toml");
    if agent_toml.exists() {
        BootstrapState::Ready
    } else {
        BootstrapState::NeedsBootstrap
    }
}

/// [v0.6.0] Bootstrap 실행 — 기본 에이전트 파일 일괄 생성.
///
/// agent_name: 에이전트 이름 (config.rs의 agent_name에서 가져옴)
/// user_name: 사용자 이름 (Bootstrap Q&A 또는 기본값)
/// language: 선호 언어 코드
pub fn run_bootstrap(
    workspace: &Path,
    agent_name: &str,
    user_name: &str,
    language: &str,
) -> Result<(), String> {
    // 1. 서브 디렉토리 생성
    let memory_dir = workspace.join("memory");
    let sessions_dir = workspace.join("sessions");
    fs::create_dir_all(&memory_dir).map_err(|e| format!("memory/ creation failed: {}", e))?;
    fs::create_dir_all(&sessions_dir).map_err(|e| format!("sessions/ creation failed: {}", e))?;

    // 2. agent.toml 생성 (페르소나)
    let mut persona = Persona::new_default(agent_name);
    persona.identity.language = language.to_string();
    persona.save(workspace)?;

    // 3. user.toml 생성 (사용자 프로필)
    let user = UserProfile::new_default(user_name, language);
    user.save(workspace)?;

    // 4. MEMORY.md 초기화 (빈 장기 기억)
    let memory_path = workspace.join("MEMORY.md");
    if !memory_path.exists() {
        let initial_memory = format!(
            "# {} Long-term Memory\n\n\
             > Curated knowledge. Agent updates this file with important learnings.\n\n\
             ---\n\n\
             *Created: {}*\n",
            agent_name,
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        );
        fs::write(&memory_path, initial_memory)
            .map_err(|e| format!("MEMORY.md creation failed: {}", e))?;
    }

    // 5. 오늘 일일 로그 초기화
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let daily_log_path = memory_dir.join(format!("{}.md", today));
    if !daily_log_path.exists() {
        let initial_log = format!(
            "# Daily Log — {}\n\n\
             - Agent `{}` initialized via bootstrap\n",
            today, agent_name
        );
        fs::write(&daily_log_path, initial_log)
            .map_err(|e| format!("Daily log creation failed: {}", e))?;
    }

    // 6. BOOTSTRAP.md 삭제 (있으면)
    let bootstrap_md = workspace.join("BOOTSTRAP.md");
    if bootstrap_md.exists() {
        let _ = fs::remove_file(&bootstrap_md);
    }

    Ok(())
}

/// [v0.6.0] BOOTSTRAP.md 시드 파일 생성 (첫 실행 시).
/// TUI 온보딩에서 사용자 입력 전에 표시 목적.
pub fn seed_bootstrap_file(workspace: &Path) -> Result<(), String> {
    let path = workspace.join("BOOTSTRAP.md");
    if path.exists() {
        return Ok(()); // 이미 존재
    }

    let content = "\
# 🐾 femtoClaw Bootstrap

Welcome! Let's set up your agent.

## Step 1: Your Name
What should I call you?

## Step 2: Language Preference
Which language do you prefer? (ko/en/ja/zh-tw/zh-cn)

## Step 3: Agent Personality
How would you like your agent to behave?
- 🎯 Professional and concise
- 🤗 Friendly and conversational
- 🧪 Technical and detailed

## Step 4: Workspace Ready
I'll create your workspace files now...

---
*This file is deleted after bootstrap completes.*
";

    fs::write(&path, content).map_err(|e| format!("BOOTSTRAP.md creation failed: {}", e))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_workspace() -> std::path::PathBuf {
        let dir = std::env::temp_dir()
            .join("femtoclaw_bootstrap_test")
            .join(format!(
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(&dir).unwrap();
        // data/와 temp/ 서브 디렉토리도 생성 (sandbox 구조 모사)
        fs::create_dir_all(dir.join("data")).unwrap();
        fs::create_dir_all(dir.join("temp")).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_check_state_needs_bootstrap() {
        let ws = temp_workspace();
        assert_eq!(check_state(&ws), BootstrapState::NeedsBootstrap);
        cleanup(&ws);
    }

    #[test]
    fn test_check_state_ready() {
        let ws = temp_workspace();
        // agent.toml 수동 생성
        fs::write(ws.join("agent.toml"), "[identity]\nname = \"Test\"").unwrap();
        assert_eq!(check_state(&ws), BootstrapState::Ready);
        cleanup(&ws);
    }

    #[test]
    fn test_run_bootstrap_creates_files() {
        let ws = temp_workspace();

        let result = run_bootstrap(&ws, "Alpha", "TestUser", "ko");
        assert!(result.is_ok(), "Bootstrap failed: {:?}", result);

        // agent.toml 존재 확인
        assert!(ws.join("agent.toml").exists());
        // user.toml 존재 확인
        assert!(ws.join("user.toml").exists());
        // MEMORY.md 존재 확인
        assert!(ws.join("MEMORY.md").exists());
        // memory/ 디렉토리 존재 확인
        assert!(ws.join("memory").is_dir());
        // sessions/ 디렉토리 존재 확인
        assert!(ws.join("sessions").is_dir());

        // agent.toml 파싱 확인
        let persona = Persona::load(&ws).unwrap();
        assert_eq!(persona.identity.name, "Alpha");
        assert_eq!(persona.identity.language, "ko");

        // 상태가 Ready로 변경됨
        assert_eq!(check_state(&ws), BootstrapState::Ready);

        cleanup(&ws);
    }

    #[test]
    fn test_seed_bootstrap_file() {
        let ws = temp_workspace();
        seed_bootstrap_file(&ws).unwrap();
        assert!(ws.join("BOOTSTRAP.md").exists());

        let content = fs::read_to_string(ws.join("BOOTSTRAP.md")).unwrap();
        assert!(content.contains("femtoClaw Bootstrap"));

        // Bootstrap 실행 후 삭제됨
        run_bootstrap(&ws, "Alpha", "User", "en").unwrap();
        assert!(!ws.join("BOOTSTRAP.md").exists());

        cleanup(&ws);
    }
}
