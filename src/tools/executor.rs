// femtoClaw — 안전 도구 실행기
// [v0.4.0] Step 9c: 도구 호출 흐름 통합 (Registry 조회 → 검증 → Jailing → 실행 → 에러 복구)
//
// 설계 원칙:
//   - 에이전트가 도구를 호출하면 이 모듈이 중앙에서 처리
//   - 존재하지 않는 도구 → 즉시 에이전트에 알림
//   - SecurityLevel::JailRequired → validate_path 호출
//   - 실행 실패 → ToolError 분류 + 사용자 친화적 메시지 생성
//   - 연속 실패 카운터 (3회 초과 시 중단 권고)

use std::path::{Path, PathBuf};

use super::guide::JailingGuide;
use super::registry::{find_tool, SecurityLevel};

/// [v0.4.0] 도구 실행 에러 유형
#[derive(Debug, Clone, PartialEq)]
pub enum ToolError {
    /// 도구가 레지스트리에 없음
    ToolNotFound(String),
    /// 필수 파라미터 누락
    MissingParam(String),
    /// 파일이 없음
    FileNotFound(String),
    /// Jailing에 의해 차단됨
    JailBlocked(String),
    /// OS 권한 부족
    PermissionDenied(String),
    /// 실행 시간 초과
    Timeout(String),
    /// 블랙리스트 명령어
    CommandBlocked(String),
    /// 3회 연속 실패
    RetryExhausted(String),
    /// 기타 에러
    Other(String),
}

impl ToolError {
    /// [v0.4.0] 에러에 대한 사용자 친화적 안내 메시지를 생성한다.
    /// 에이전트가 사용자에게 보여줄 메시지.
    pub fn user_message(&self) -> String {
        match self {
            ToolError::ToolNotFound(name) => {
                format!(
                    "'{}'이라는 도구는 존재하지 않습니다. \
                     사용 가능한 도구: file_read, file_write, file_list, sleep, print",
                    name
                )
            }
            ToolError::MissingParam(param) => {
                format!("필수 파라미터 '{}'가 누락되었습니다.", param)
            }
            ToolError::FileNotFound(path) => {
                format!(
                    "파일 '{}'을(를) 찾을 수 없습니다. \
                     경로를 확인해주세요. file_list('.')으로 현재 파일 목록을 확인할 수 있습니다.",
                    path
                )
            }
            ToolError::JailBlocked(detail) => JailingGuide::explain_block(detail),
            ToolError::PermissionDenied(path) => {
                format!(
                    "파일 '{}'에 대한 접근 권한이 없습니다. \
                     다른 경로를 시도하거나, 파일 권한을 확인해주세요.",
                    path
                )
            }
            ToolError::Timeout(detail) => {
                format!(
                    "작업이 시간 제한을 초과했습니다: {}. \
                     더 작은 작업으로 나눠서 시도해보세요.",
                    detail
                )
            }
            ToolError::CommandBlocked(cmd) => {
                format!(
                    "보안 정책으로 '{}' 명령어는 사용할 수 없습니다. \
                     이것은 시스템을 보호하기 위한 안전장치입니다.",
                    cmd
                )
            }
            ToolError::RetryExhausted(tool_id) => {
                format!(
                    "도구 '{}'가 3회 연속 실패했습니다. \
                     자동 재시도를 중단합니다. \
                     문제가 지속되면 도움을 요청해주세요.",
                    tool_id
                )
            }
            ToolError::Other(msg) => {
                format!("도구 실행 중 오류가 발생했습니다: {}", msg)
            }
        }
    }

    /// 보안 이벤트 여부
    pub fn is_security_event(&self) -> bool {
        matches!(
            self,
            ToolError::JailBlocked(_) | ToolError::CommandBlocked(_)
        )
    }
}

/// [v0.4.0] 도구 실행 결과
#[derive(Debug, Clone)]
pub struct ToolResult {
    /// 도구 ID
    pub tool_id: String,
    /// 성공 여부
    pub success: bool,
    /// 결과 값 (성공 시)
    pub output: Option<String>,
    /// 에러 (실패 시)
    pub error: Option<ToolError>,
    /// 보안 이벤트 여부
    pub security_event: bool,
}

/// [v0.4.0] 안전 도구 실행기
pub struct ToolExecutor {
    /// workspace 루트 (Jailing 경계)
    workspace: PathBuf,
    /// 도구별 연속 실패 카운터 (tool_id → 실패 횟수)
    failure_counts: std::collections::HashMap<String, u8>,
}

impl ToolExecutor {
    /// 새 실행기 생성
    pub fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            failure_counts: std::collections::HashMap::new(),
        }
    }

    /// [v0.4.0] 도구를 실행한다.
    ///
    /// 흐름: Registry 조회 → 연속 실패 확인 → 보안 검증 → 실행 → 결과 반환
    pub fn execute(&mut self, tool_id: &str, params: &[(&str, &str)]) -> ToolResult {
        // 1. Registry 조회
        let tool_def = match find_tool(tool_id) {
            Some(t) => t,
            None => {
                return ToolResult {
                    tool_id: tool_id.to_string(),
                    success: false,
                    output: None,
                    error: Some(ToolError::ToolNotFound(tool_id.to_string())),
                    security_event: false,
                };
            }
        };

        // 2. 연속 실패 확인 (3회 초과 시 중단)
        let fail_count = self.failure_counts.get(tool_id).copied().unwrap_or(0);
        if fail_count >= 3 {
            return ToolResult {
                tool_id: tool_id.to_string(),
                success: false,
                output: None,
                error: Some(ToolError::RetryExhausted(tool_id.to_string())),
                security_event: false,
            };
        }

        // 3. 필수 파라미터 검증
        for p in tool_def.params {
            if p.required && !params.iter().any(|(k, _)| *k == p.name) {
                let err = ToolError::MissingParam(p.name.to_string());
                self.record_failure(tool_id);
                return ToolResult {
                    tool_id: tool_id.to_string(),
                    success: false,
                    output: None,
                    error: Some(err),
                    security_event: false,
                };
            }
        }

        // 4. 보안 검증 (JailRequired인 경우)
        // ../ 패턴 차단 + workspace 접두사 검증
        if tool_def.security_level == SecurityLevel::JailRequired {
            if let Some(path_param) = params.iter().find(|(k, _)| *k == "path" || *k == "dir") {
                let path_str = path_param.1;
                // ../ 패턴 차단
                if path_str.contains("..") {
                    let err = ToolError::JailBlocked(format!(
                        "BLOCKED: 디렉토리 순회(../) 금지 — {}",
                        path_str
                    ));
                    self.record_failure(tool_id);
                    return ToolResult {
                        tool_id: tool_id.to_string(),
                        success: false,
                        output: None,
                        error: Some(err),
                        security_event: true,
                    };
                }
                // 절대 경로 차단 (workspace 밖 접근 방지)
                let p = std::path::Path::new(path_str);
                if p.is_absolute() {
                    let err = ToolError::JailBlocked(format!(
                        "BLOCKED: 경로 탈출 시도 — 절대 경로 '{}' 사용 금지, 상대 경로만 사용하세요",
                        path_str
                    ));
                    self.record_failure(tool_id);
                    return ToolResult {
                        tool_id: tool_id.to_string(),
                        success: false,
                        output: None,
                        error: Some(err),
                        security_event: true,
                    };
                }
            }
        }

        // 5. 실행
        let result = self.dispatch(tool_id, params);

        match result {
            Ok(output) => {
                // 성공 — 실패 카운터 리셋
                self.failure_counts.remove(tool_id);
                ToolResult {
                    tool_id: tool_id.to_string(),
                    success: true,
                    output: Some(output),
                    error: None,
                    security_event: false,
                }
            }
            Err(err) => {
                let security_event = err.is_security_event();
                self.record_failure(tool_id);
                ToolResult {
                    tool_id: tool_id.to_string(),
                    success: false,
                    output: None,
                    error: Some(err),
                    security_event,
                }
            }
        }
    }

    /// 연속 실패 카운터 증가
    fn record_failure(&mut self, tool_id: &str) {
        let count = self.failure_counts.entry(tool_id.to_string()).or_insert(0);
        *count += 1;
    }

    /// 실패 카운터 리셋 (외부에서 수동 리셋용)
    pub fn reset_failures(&mut self, tool_id: &str) {
        self.failure_counts.remove(tool_id);
    }

    /// 실제 도구 실행 디스패치
    fn dispatch(&self, tool_id: &str, params: &[(&str, &str)]) -> Result<String, ToolError> {
        match tool_id {
            "file_read" => {
                let path = get_param(params, "path")?;
                let full_path = self.workspace.join(path);
                read_file_safe(&full_path)
            }
            "file_write" => {
                let path = get_param(params, "path")?;
                let content = get_param(params, "content")?;
                let full_path = self.workspace.join(path);
                write_file_safe(&full_path, content)
            }
            "file_list" => {
                let dir = get_param(params, "dir")?;
                let full_path = self.workspace.join(dir);
                list_dir_safe(&full_path)
            }
            "sleep" => {
                let ms_str = get_param(params, "ms")?;
                let ms: i64 = ms_str.parse().unwrap_or(0);
                let clamped = ms.clamp(0, 5000) as u64;
                std::thread::sleep(std::time::Duration::from_millis(clamped));
                Ok(format!("{}ms 대기 완료", clamped))
            }
            "print" => {
                let msg = get_param(params, "msg")?;
                Ok(msg.to_string())
            }
            _ => Err(ToolError::ToolNotFound(tool_id.to_string())),
        }
    }
}

// === 내부 도구 구현 함수 ===

fn get_param<'a>(params: &'a [(&str, &str)], name: &str) -> Result<&'a str, ToolError> {
    params
        .iter()
        .find(|(k, _)| *k == name)
        .map(|(_, v)| *v)
        .ok_or_else(|| ToolError::MissingParam(name.to_string()))
}

fn read_file_safe(path: &Path) -> Result<String, ToolError> {
    if !path.exists() {
        return Err(ToolError::FileNotFound(path.display().to_string()));
    }
    std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            ToolError::PermissionDenied(path.display().to_string())
        } else {
            ToolError::Other(format!("파일 읽기 실패: {}", e))
        }
    })
}

fn write_file_safe(path: &Path, content: &str) -> Result<String, ToolError> {
    // 부모 디렉토리 자동 생성
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| ToolError::Other(format!("디렉토리 생성 실패: {}", e)))?;
    }
    std::fs::write(path, content).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            ToolError::PermissionDenied(path.display().to_string())
        } else {
            ToolError::Other(format!("파일 쓰기 실패: {}", e))
        }
    })?;
    Ok(format!("파일 저장 완료: {}", path.display()))
}

fn list_dir_safe(path: &Path) -> Result<String, ToolError> {
    if !path.exists() {
        return Err(ToolError::FileNotFound(path.display().to_string()));
    }
    let entries = std::fs::read_dir(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::PermissionDenied {
            ToolError::PermissionDenied(path.display().to_string())
        } else {
            ToolError::Other(format!("디렉토리 탐색 실패: {}", e))
        }
    })?;

    let mut names: Vec<String> = Vec::new();
    for entry in entries.flatten() {
        if let Some(name) = entry.file_name().to_str() {
            names.push(name.to_string());
        }
    }
    names.sort();
    Ok(names.join(", "))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn temp_workspace() -> PathBuf {
        let dir = std::env::temp_dir()
            .join("femtoclaw_executor_test")
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
    fn test_file_read_write() {
        let ws = temp_workspace();
        let mut exec = ToolExecutor::new(ws.clone());

        // 파일 쓰기
        let result = exec.execute("file_write", &[("path", "test.txt"), ("content", "hello")]);
        assert!(result.success);
        assert!(result.output.unwrap().contains("저장 완료"));

        // 파일 읽기
        let result = exec.execute("file_read", &[("path", "test.txt")]);
        assert!(result.success);
        assert_eq!(result.output.unwrap(), "hello");

        cleanup(&ws);
    }

    #[test]
    fn test_file_not_found() {
        let ws = temp_workspace();
        let mut exec = ToolExecutor::new(ws.clone());

        let result = exec.execute("file_read", &[("path", "nonexistent.txt")]);
        assert!(!result.success);
        assert!(matches!(result.error, Some(ToolError::FileNotFound(_))));

        let msg = result.error.unwrap().user_message();
        assert!(msg.contains("찾을 수 없습니다"));

        cleanup(&ws);
    }

    #[test]
    fn test_tool_not_found() {
        let ws = temp_workspace();
        let mut exec = ToolExecutor::new(ws.clone());

        let result = exec.execute("magic_wand", &[]);
        assert!(!result.success);
        assert!(matches!(result.error, Some(ToolError::ToolNotFound(_))));

        cleanup(&ws);
    }

    #[test]
    fn test_missing_param() {
        let ws = temp_workspace();
        let mut exec = ToolExecutor::new(ws.clone());

        // file_read에 path 없이 호출
        let result = exec.execute("file_read", &[]);
        assert!(!result.success);
        assert!(matches!(result.error, Some(ToolError::MissingParam(_))));

        cleanup(&ws);
    }

    #[test]
    fn test_retry_exhaustion() {
        let ws = temp_workspace();
        let mut exec = ToolExecutor::new(ws.clone());

        // 3회 연속 실패
        for _ in 0..3 {
            let _ = exec.execute("file_read", &[("path", "nope.txt")]);
        }

        // 4번째는 RetryExhausted
        let result = exec.execute("file_read", &[("path", "nope.txt")]);
        assert!(!result.success);
        assert!(matches!(result.error, Some(ToolError::RetryExhausted(_))));

        // 리셋 후 재시도 가능
        exec.reset_failures("file_read");
        let result = exec.execute("file_read", &[("path", "nope.txt")]);
        assert!(!result.success);
        // RetryExhausted가 아닌 FileNotFound
        assert!(matches!(result.error, Some(ToolError::FileNotFound(_))));

        cleanup(&ws);
    }

    #[test]
    fn test_file_list() {
        let ws = temp_workspace();
        let mut exec = ToolExecutor::new(ws.clone());

        // 파일 두 개 생성
        fs::write(ws.join("alpha.txt"), "a").unwrap();
        fs::write(ws.join("beta.txt"), "b").unwrap();

        let result = exec.execute("file_list", &[("dir", ".")]);
        assert!(result.success);
        let output = result.output.unwrap();
        assert!(output.contains("alpha.txt"));
        assert!(output.contains("beta.txt"));

        cleanup(&ws);
    }

    #[test]
    fn test_print_and_sleep() {
        let ws = temp_workspace();
        let mut exec = ToolExecutor::new(ws.clone());

        let result = exec.execute("print", &[("msg", "안녕")]);
        assert!(result.success);
        assert_eq!(result.output.unwrap(), "안녕");

        let result = exec.execute("sleep", &[("ms", "10")]);
        assert!(result.success);

        cleanup(&ws);
    }

    #[test]
    fn test_security_event_flag() {
        let ws = temp_workspace();
        let mut exec = ToolExecutor::new(ws.clone());

        // jail 위반 시도 (../ 사용)
        let result = exec.execute("file_read", &[("path", "../../etc/passwd")]);
        assert!(!result.success);
        assert!(result.security_event);

        cleanup(&ws);
    }
}
