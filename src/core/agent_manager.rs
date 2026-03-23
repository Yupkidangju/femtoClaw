// femtoClaw — 멀티 에이전트 매니저
// [v0.3.0] Step 8a/8b: 에이전트별 DB + workspace 자동 생성 및 격리 관리.
//
// 설계 원칙:
//   - 각 에이전트는 ~/.femtoclaw/agents/<id>/ 하위에 격리된 DB와 workspace를 갖는다.
//   - 스킬은 전체 공유 (skills/core/, skills/user/)
//   - 에이전트 추가 시 디렉토리 자동 생성
//   - 최대 3개 에이전트 제한

use std::path::{Path, PathBuf};

/// [v0.3.0] 에이전트별 격리된 경로 정보
#[derive(Debug, Clone)]
pub struct AgentPaths {
    /// 에이전트 ID
    pub id: u8,
    /// 에이전트 루트 (예: ~/.femtoclaw/agents/1/)
    pub root: PathBuf,
    /// 에이전트 DB 파일 경로
    pub db_file: PathBuf,
    /// 에이전트 DB 디렉토리
    pub db_dir: PathBuf,
    /// 에이전트 workspace (Jailing 경계)
    pub workspace: PathBuf,
    /// workspace/data
    pub workspace_data: PathBuf,
    /// workspace/temp (자동 소멸)
    pub workspace_temp: PathBuf,
}

impl AgentPaths {
    /// [v0.3.0] 에이전트 경로를 계산한다.
    /// sandbox_root: ~/.femtoclaw/
    pub fn new(sandbox_root: &Path, agent_id: u8) -> Self {
        let root = sandbox_root.join("agents").join(agent_id.to_string());
        let db_dir = root.join("db");

        Self {
            id: agent_id,
            db_file: db_dir.join("femto_state.db"),
            db_dir,
            workspace: root.join("workspace"),
            workspace_data: root.join("workspace").join("data"),
            workspace_temp: root.join("workspace").join("temp"),
            root,
        }
    }

    /// [v0.3.0] 에이전트 디렉토리 구조를 생성한다.
    pub fn ensure_dirs(&self) -> Result<(), String> {
        let dirs = [&self.db_dir, &self.workspace_data, &self.workspace_temp];
        for dir in dirs {
            std::fs::create_dir_all(dir)
                .map_err(|e| format!("에이전트 #{} 디렉토리 생성 실패: {}", self.id, e))?;
        }
        Ok(())
    }
}

/// [v0.3.0] 멀티 에이전트 매니저 — 에이전트별 경로 관리
pub struct AgentManager {
    /// 샌드박스 루트 경로
    sandbox_root: PathBuf,
    /// 에이전트별 경로 캐시
    agent_paths: Vec<AgentPaths>,
}

impl AgentManager {
    /// [v0.3.0] 매니저 생성 — 등록된 에이전트 ID 목록으로 초기화
    pub fn new(sandbox_root: PathBuf, agent_ids: &[u8]) -> Result<Self, String> {
        let mut agent_paths = Vec::new();

        for &id in agent_ids {
            let paths = AgentPaths::new(&sandbox_root, id);
            paths.ensure_dirs()?;
            agent_paths.push(paths);
        }

        Ok(Self {
            sandbox_root,
            agent_paths,
        })
    }

    /// [v0.3.0] 에이전트 경로 가져오기
    pub fn get_paths(&self, agent_id: u8) -> Option<&AgentPaths> {
        self.agent_paths.iter().find(|p| p.id == agent_id)
    }

    /// [v0.3.0] 새 에이전트 추가 (디렉토리 생성 포함)
    pub fn add_agent(&mut self, agent_id: u8) -> Result<&AgentPaths, String> {
        if self.agent_paths.iter().any(|p| p.id == agent_id) {
            return Err(format!("에이전트 #{}는 이미 존재합니다", agent_id));
        }
        if self.agent_paths.len() >= 3 {
            return Err("에이전트는 최대 3개까지 등록 가능합니다".to_string());
        }

        let paths = AgentPaths::new(&self.sandbox_root, agent_id);
        paths.ensure_dirs()?;
        self.agent_paths.push(paths);

        Ok(self.agent_paths.last().unwrap())
    }

    /// 등록된 에이전트 수
    pub fn count(&self) -> usize {
        self.agent_paths.len()
    }

    /// 모든 에이전트 ID 목록
    pub fn agent_ids(&self) -> Vec<u8> {
        self.agent_paths.iter().map(|p| p.id).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_sandbox() -> PathBuf {
        let dir = std::env::temp_dir()
            .join("femtoclaw_agent_test")
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
    fn test_agent_paths() {
        let sandbox = temp_sandbox();
        let paths = AgentPaths::new(&sandbox, 1);

        assert!(paths.root.ends_with("1"));
        assert!(paths.db_file.to_string_lossy().contains("femto_state.db"));
        assert!(paths.workspace.to_string_lossy().contains("workspace"));
        cleanup(&sandbox);
    }

    #[test]
    fn test_ensure_dirs() {
        let sandbox = temp_sandbox();
        let paths = AgentPaths::new(&sandbox, 2);
        paths.ensure_dirs().unwrap();

        assert!(paths.db_dir.exists());
        assert!(paths.workspace_data.exists());
        assert!(paths.workspace_temp.exists());
        cleanup(&sandbox);
    }

    #[test]
    fn test_agent_manager() {
        let sandbox = temp_sandbox();
        let mgr = AgentManager::new(sandbox.clone(), &[1, 2]).unwrap();

        assert_eq!(mgr.count(), 2);
        assert!(mgr.get_paths(1).is_some());
        assert!(mgr.get_paths(2).is_some());
        assert!(mgr.get_paths(3).is_none());
        assert_eq!(mgr.agent_ids(), vec![1, 2]);
        cleanup(&sandbox);
    }

    #[test]
    fn test_add_agent() {
        let sandbox = temp_sandbox();
        let mut mgr = AgentManager::new(sandbox.clone(), &[1]).unwrap();

        // 에이전트 추가
        mgr.add_agent(2).unwrap();
        assert_eq!(mgr.count(), 2);
        assert!(mgr.get_paths(2).unwrap().db_dir.exists());

        // 3번째 추가
        mgr.add_agent(3).unwrap();
        assert_eq!(mgr.count(), 3);

        // 4번째는 실패
        assert!(mgr.add_agent(4).is_err());

        // 중복 추가 실패
        assert!(mgr.add_agent(1).is_err());

        cleanup(&sandbox);
    }

    #[test]
    fn test_config_multi_agent() {
        use crate::config::AppConfig;

        let mut config = AppConfig::default();
        assert_eq!(config.agents.len(), 1);
        assert_eq!(config.active_agent_id, 1);

        // 에이전트 추가
        let id2 = config.add_agent("Beta").unwrap();
        assert_eq!(id2, 2);
        assert_eq!(config.agents.len(), 2);

        let id3 = config.add_agent("Gamma").unwrap();
        assert_eq!(id3, 3);

        // 4번째 실패
        assert!(config.add_agent("Delta").is_err());

        // 전환
        config.switch_agent(2).unwrap();
        assert_eq!(config.active_agent_id, 2);
        assert_eq!(config.active_agent().unwrap().name, "Beta");

        // 존재하지 않는 에이전트 전환 실패
        assert!(config.switch_agent(99).is_err());
    }
}
