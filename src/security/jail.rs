// femtoClaw — Path Jailing & 블랙리스트 커맨드 가드
// [v0.1.0] Step 5: 에이전트의 파일 I/O를 workspace/로 강제 제한하고,
// 파괴적 시스템 명령어를 사전 차단한다.
//
// 보안 원칙:
//   1. 모든 파일 경로는 canonicalize() 후 workspace 접두사를 검증
//   2. ../ 디렉토리 순회(Directory Traversal)는 정규화 후 차단
//   3. 심볼릭 링크는 실제 경로로 해석 후 검증
//   4. rm -rf, format, mkfs, sudo 등 파괴적 명령어를 블랙리스트로 차단
//   5. 모든 차단 이벤트는 JailViolation 에러로 반환

use std::path::{Path, PathBuf};

/// Jail 위반 유형
#[derive(Debug, Clone, PartialEq)]
pub enum JailViolation {
    /// 경로가 workspace 밖을 가리킴
    PathEscape(String),
    /// 블랙리스트 명령어 사용 시도
    BlacklistedCommand(String),
    /// 경로 정규화 실패
    InvalidPath(String),
}

impl std::fmt::Display for JailViolation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            JailViolation::PathEscape(p) => write!(f, "BLOCKED: 경로 탈출 시도 — {}", p),
            JailViolation::BlacklistedCommand(c) => write!(f, "BLOCKED: 금지 명령어 — {}", c),
            JailViolation::InvalidPath(p) => write!(f, "BLOCKED: 유효하지 않은 경로 — {}", p),
        }
    }
}

/// 파괴적 명령어 블랙리스트 (시스템 보호)
const BLACKLISTED_COMMANDS: &[&str] = &[
    // 파일 시스템 파괴
    "rm -rf",
    "rm -fr",
    "rmdir /s",
    "del /f /s /q",
    // 디스크 포맷
    "format",
    "mkfs",
    "fdisk",
    "diskpart",
    // 권한 에스컬레이션
    "sudo",
    "su ",
    "runas",
    // 시스템 종료/재시작
    "shutdown",
    "reboot",
    "init 0",
    "init 6",
    // 레지스트리/시스템 파일
    "reg delete",
    "reg add",
    // 네트워크 파괴
    "iptables -F",
    "netsh",
    // 위험한 쉘 명령
    ":(){ :|:& };:", // Fork bomb
    "dd if=",
    "> /dev/sda",
];

/// [v0.1.0] 경로가 workspace 내부에 있는지 검증한다.
/// canonicalize()로 심볼릭 링크를 해석하고 ../ 순회를 정규화한 뒤,
/// workspace 접두사를 포함하는지 확인한다.
///
/// workspace가 아직 존재하지 않으면 (첫 실행 전) 문자열 비교로 검증.
pub fn validate_path(path: &Path, workspace: &Path) -> Result<PathBuf, JailViolation> {
    // workspace 절대 경로 확보
    let workspace_canonical = if workspace.exists() {
        workspace
            .canonicalize()
            .map_err(|e| JailViolation::InvalidPath(format!("{}: {}", workspace.display(), e)))?
    } else {
        workspace.to_path_buf()
    };

    // 대상 경로 정규화
    let target = if path.is_absolute() {
        path.to_path_buf()
    } else {
        workspace.join(path)
    };

    let target_canonical = if target.exists() {
        target
            .canonicalize()
            .map_err(|e| JailViolation::InvalidPath(format!("{}: {}", target.display(), e)))?
    } else {
        // 파일이 아직 없으면 (생성 전) 부모 기준 검증
        // ../ 패턴 감지
        let path_str = target.to_string_lossy();
        if path_str.contains("..") {
            return Err(JailViolation::PathEscape(path_str.to_string()));
        }
        target
    };

    // workspace 접두사 검증
    if target_canonical.starts_with(&workspace_canonical) {
        Ok(target_canonical)
    } else {
        Err(JailViolation::PathEscape(format!(
            "{} → workspace({}) 밖",
            target_canonical.display(),
            workspace_canonical.display()
        )))
    }
}

/// [v0.1.0] 명령어 문자열이 블랙리스트에 해당하는지 검사한다.
/// 대소문자 무관하게 검사하여 우회를 차단한다.
pub fn validate_command(command: &str) -> Result<(), JailViolation> {
    let lower = command.to_lowercase();
    for &blocked in BLACKLISTED_COMMANDS {
        if lower.contains(&blocked.to_lowercase()) {
            return Err(JailViolation::BlacklistedCommand(blocked.to_string()));
        }
    }
    Ok(())
}

/// [v0.1.0] temp/ 디렉토리의 파일을 자동 소멸한다.
/// workspace/temp/ 내부의 모든 파일/디렉토리를 삭제.
pub fn cleanup_temp(workspace: &Path) -> Result<usize, String> {
    let temp_dir = workspace.join("temp");
    if !temp_dir.exists() {
        return Ok(0);
    }

    let mut count = 0;
    let entries =
        std::fs::read_dir(&temp_dir).map_err(|e| format!("temp 디렉토리 읽기 실패: {}", e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("항목 읽기 실패: {}", e))?;
        let path = entry.path();

        if path.is_dir() {
            std::fs::remove_dir_all(&path)
                .map_err(|e| format!("디렉토리 삭제 실패 {}: {}", path.display(), e))?;
        } else {
            std::fs::remove_file(&path)
                .map_err(|e| format!("파일 삭제 실패 {}: {}", path.display(), e))?;
        }
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// 임시 workspace 생성 헬퍼
    fn setup_workspace() -> PathBuf {
        let dir = std::env::temp_dir()
            .join("femtoclaw_jail_test")
            .join(format!(
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos()
            ));
        fs::create_dir_all(dir.join("data")).unwrap();
        fs::create_dir_all(dir.join("temp")).unwrap();
        dir
    }

    fn cleanup(dir: &Path) {
        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn test_valid_path_inside_workspace() {
        let ws = setup_workspace();
        // data/ 내에 실제 파일 생성 (canonicalize가 작동하려면 파일이 필요)
        fs::write(ws.join("data").join("test.txt"), "hello").unwrap();
        let result = validate_path(Path::new("data/test.txt"), &ws);
        assert!(result.is_ok(), "결과: {:?}", result);
        cleanup(&ws);
    }

    #[test]
    fn test_path_traversal_blocked() {
        let ws = setup_workspace();
        // ../ 순회 시도 차단
        let result = validate_path(Path::new("../../etc/passwd"), &ws);
        assert!(result.is_err());
        if let Err(JailViolation::PathEscape(_)) = result {
        } else {
            panic!("PathEscape 에러여야 함");
        }
        cleanup(&ws);
    }

    #[test]
    fn test_absolute_path_outside_workspace() {
        let ws = setup_workspace();
        // 절대 경로로 workspace 밖 접근 시도
        let outside = std::env::temp_dir().join("outside_workspace.txt");
        fs::write(&outside, "test").unwrap();
        let result = validate_path(&outside, &ws);
        assert!(result.is_err());
        let _ = fs::remove_file(&outside);
        cleanup(&ws);
    }

    #[test]
    fn test_blacklisted_commands() {
        // 파괴적 명령어 차단
        assert!(validate_command("rm -rf /").is_err());
        assert!(validate_command("sudo apt install").is_err());
        assert!(validate_command("FORMAT C:").is_err()); // 대소문자 무관
        assert!(validate_command("mkfs.ext4 /dev/sda").is_err());
        assert!(validate_command("shutdown -h now").is_err());
        assert!(validate_command("dd if=/dev/zero of=/dev/sda").is_err());
    }

    #[test]
    fn test_safe_commands() {
        // 안전한 명령어 허용
        assert!(validate_command("ls -la").is_ok());
        assert!(validate_command("cat file.txt").is_ok());
        assert!(validate_command("echo hello").is_ok());
        assert!(validate_command("grep pattern file.txt").is_ok());
        assert!(validate_command("python script.py").is_ok());
    }

    #[test]
    fn test_cleanup_temp() {
        let ws = setup_workspace();
        let temp = ws.join("temp");

        // temp에 테스트 파일 생성
        fs::write(temp.join("file1.txt"), "data1").unwrap();
        fs::write(temp.join("file2.txt"), "data2").unwrap();
        fs::create_dir(temp.join("subdir")).unwrap();
        fs::write(temp.join("subdir").join("nested.txt"), "nested").unwrap();

        let count = cleanup_temp(&ws).unwrap();
        assert_eq!(count, 3); // file1, file2, subdir

        // temp 디렉토리 자체는 남아있어야 함
        assert!(temp.exists());
        // 내부는 비어있어야 함
        assert_eq!(fs::read_dir(&temp).unwrap().count(), 0);

        cleanup(&ws);
    }
}
