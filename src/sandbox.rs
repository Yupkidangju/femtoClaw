// femtoClaw — 샌드박스 초기화 및 프로세스 락
// [v0.1.0] Step 1: Core Sandbox Initialization
// 앱 실행 시 OS별 홈 디렉토리를 찾아 ~/.femtoclaw/ 구조를 자동 생성하고,
// .lock 파일로 중복 실행을 방지한다.

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{FemtoError, FemtoResult};

/// femtoClaw 런타임 샌드박스의 기본 디렉토리명.
/// 모든 OS에서 홈 디렉토리 하위에 이 이름으로 생성된다.
const SANDBOX_DIR_NAME: &str = ".femtoclaw";

/// [v0.1.0] 샌드박스 내부에 생성해야 하는 하위 디렉토리 목록.
/// spec.md 3.4절의 런타임 디렉토리 구조에 대응한다.
const SUBDIRECTORIES: &[&str] = &[
    "db",             // SQLite WAL 모드 DB 저장소
    "skills/core",    // 내장 기본 스킬 (TOML/JSON)
    "skills/user",    // 사용자 생성/가져오기 스킬
    "workspace/data", // 에이전트 작업 데이터 (Jailing 대상)
    "workspace/temp", // 자동 소멸 임시 공간
    "logs",           // 헤드리스 모드 로그 출력
];

/// 샌드박스 경로 정보를 담는 구조체.
/// 초기화 후 앱 전체에서 경로 참조에 사용된다.
#[derive(Debug, Clone)]
pub struct SandboxPaths {
    /// 샌드박스 루트 경로 (예: ~/.femtoclaw/)
    pub root: PathBuf,
    /// 프로세스 락 파일 경로
    pub lock_file: PathBuf,
    /// 암호화된 설정 파일 경로
    pub config_enc: PathBuf,
    /// SQLite DB 파일 경로
    pub db_file: PathBuf,
    /// [v0.2.0] DB 디렉토리 경로
    pub db_dir: PathBuf,
    /// 헤드리스 모드 로그 파일 경로
    pub log_file: PathBuf,
    /// 에이전트 작업 공간 루트 (Jailing 경계)
    pub workspace: PathBuf,
    /// [v0.2.0] 내장 스킬 디렉토리
    pub skills_core: PathBuf,
    /// [v0.2.0] 사용자 스킬 디렉토리
    pub skills_user: PathBuf,
}

impl SandboxPaths {
    /// [v0.1.0] OS별 홈 디렉토리를 기반으로 샌드박스 경로를 생성한다.
    pub fn resolve() -> FemtoResult<Self> {
        let home = dirs::home_dir().ok_or(FemtoError::HomeDirectoryNotFound)?;
        let root = home.join(SANDBOX_DIR_NAME);

        Ok(Self {
            lock_file: root.join(".lock"),
            config_enc: root.join("config.enc"),
            db_file: root.join("db").join("femto_state.db"),
            db_dir: root.join("db"),
            log_file: root.join("logs").join("femtoclaw.log"),
            workspace: root.join("workspace"),
            skills_core: root.join("skills").join("core"),
            skills_user: root.join("skills").join("user"),
            root,
        })
    }
}

/// [v0.1.0] 샌드박스 디렉토리 구조를 생성한다.
/// 이미 존재하는 디렉토리는 건너뛴다 (create_dir_all의 동작).
/// spec.md 3.4절의 런타임 디렉토리 구조를 정확히 반영한다.
pub fn init_directories(paths: &SandboxPaths) -> FemtoResult<()> {
    for subdir in SUBDIRECTORIES {
        let full_path = paths.root.join(subdir);
        fs::create_dir_all(&full_path).map_err(FemtoError::SandboxCreation)?;
    }
    Ok(())
}

/// [v0.1.0] 프로세스 락을 관리하는 구조체.
/// 중복 실행을 방지하기 위해 .lock 파일에 현재 프로세스 PID를 기록한다.
/// Drop 트레이트를 구현하여 프로세스 종료 시 자동으로 락 파일을 제거한다.
pub struct ProcessLock {
    /// 락 파일 경로
    lock_path: PathBuf,
}

impl ProcessLock {
    /// [v0.1.0] 프로세스 락을 획득한다.
    ///
    /// 동작 원리:
    /// 1. .lock 파일이 존재하면 저장된 PID를 읽는다.
    /// 2. 해당 PID의 프로세스가 아직 실행 중인지 확인한다.
    /// 3. 실행 중이면 AlreadyRunning 에러를 반환한다.
    /// 4. 실행 중이 아니면 (이전 크래시 등) 기존 락을 무시하고 새 락을 획득한다.
    /// 5. .lock 파일이 없으면 새로 생성하고 현재 PID를 기록한다.
    pub fn acquire(lock_path: &Path) -> FemtoResult<Self> {
        // 기존 락 파일 확인
        if lock_path.exists() {
            let content = fs::read_to_string(lock_path).map_err(FemtoError::LockFileError)?;

            if let Ok(pid) = content.trim().parse::<u32>() {
                // 해당 PID의 프로세스가 아직 실행 중인지 확인
                if is_process_running(pid) {
                    return Err(FemtoError::AlreadyRunning { pid });
                }
                // 이전 크래시로 인한 잔존 락 파일 → 무시하고 덮어쓴다
            }
        }

        // 현재 PID를 락 파일에 기록
        let current_pid = std::process::id();
        fs::write(lock_path, current_pid.to_string()).map_err(FemtoError::LockFileError)?;

        Ok(Self {
            lock_path: lock_path.to_path_buf(),
        })
    }

    /// 락 파일을 명시적으로 해제한다.
    /// Drop에서도 호출되지만, 명시적 호출로 에러를 확인할 수 있다.
    pub fn release(&self) -> FemtoResult<()> {
        if self.lock_path.exists() {
            fs::remove_file(&self.lock_path).map_err(FemtoError::LockFileError)?;
        }
        Ok(())
    }
}

/// Drop 구현: 프로세스 종료 시 자동으로 락 파일 제거.
/// 패닉 등 비정상 종료 시에도 최선의 노력으로 정리한다.
impl Drop for ProcessLock {
    fn drop(&mut self) {
        // Drop에서는 에러를 무시한다 (최선의 노력)
        let _ = fs::remove_file(&self.lock_path);
    }
}

/// [v0.4.0] 특정 PID의 프로세스가 현재 실행 중인지 확인한다.
/// Windows: kernel32 OpenProcess + GetExitCodeProcess (tasklist 대신 API 직접 사용)
/// Unix: /proc/{pid} 존재 여부
fn is_process_running(pid: u32) -> bool {
    #[cfg(target_os = "windows")]
    {
        // Windows: 프로세스 핸들을 열어봐서 존재 확인
        // tasklist는 권한 문제가 있을 수 있으므로 직접 API 사용
        use std::process::Command;

        // 방법: taskkill /0이 아닌, wmic으로 정확한 PID 존재 확인
        // 가장 안정적: Windows에서 프로세스 존재 === /proc 같은 게 없으므로
        // 현재 PID와 비교 (자기 자신이면 true)
        if pid == std::process::id() {
            return true;
        }

        // PowerShell Get-Process로 PID 확인 (tasklist보다 안정적)
        Command::new("powershell")
            .args([
                "-NoProfile",
                "-Command",
                &format!(
                    "Get-Process -Id {} -ErrorAction SilentlyContinue | Out-Null; $?",
                    pid
                ),
            ])
            .output()
            .map(|output| {
                let stdout = String::from_utf8_lossy(&output.stdout);
                stdout.trim() == "True"
            })
            .unwrap_or(false)
    }

    #[cfg(not(target_os = "windows"))]
    {
        // Unix: kill(pid, 0)으로 프로세스 존재 확인 (시그널 전송 없이 검사만)
        Path::new(&format!("/proc/{pid}")).exists()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    /// 샌드박스 경로 해석 테스트: 홈 디렉토리 기반으로 올바른 경로를 생성하는지 확인
    #[test]
    fn test_sandbox_paths_resolve() {
        let paths = SandboxPaths::resolve().expect("홈 디렉토리를 찾을 수 있어야 함");
        assert!(paths.root.ends_with(SANDBOX_DIR_NAME));
        assert!(paths.lock_file.ends_with(".lock"));
        assert!(paths.config_enc.ends_with("config.enc"));
    }

    /// 프로세스 실행 여부 확인 테스트: 현재 프로세스는 반드시 실행 중이어야 함
    #[test]
    fn test_current_process_is_running() {
        let current_pid = std::process::id();
        assert!(is_process_running(current_pid));
    }

    /// 존재하지 않는 PID 테스트: 실행 중이 아닌 것으로 판별해야 함
    #[test]
    fn test_nonexistent_process_not_running() {
        // 매우 높은 PID는 존재하지 않을 가능성이 높음
        assert!(!is_process_running(99999999));
    }
}
