// femtoClaw — 도구 레지스트리
// [v0.4.0] Step 9a: 내장 도구 5종의 정의·파라미터·제약·에러 가이드를 구조화한다.
//
// 설계 원칙:
//   - 모든 도구는 ToolDef로 정의 (이름, 설명, 파라미터, 제약, 에러 가이드)
//   - SecurityLevel로 Jailing 연동 수준을 구분
//   - BUILTIN_TOOLS 상수로 컴파일 타임에 등록
//   - 에이전트(LLM)에게 주입할 정보의 근거 자료

/// [v0.4.0] 도구 보안 수준
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SecurityLevel {
    /// 부작용 없음 — 검증 없이 실행 (print, sleep)
    Safe,
    /// Jailing 검증 필수 — 파일 I/O 등 workspace 경계 검사
    JailRequired,
    /// 추가 승인 필요 — 향후 외부 API, 시스템 명령 등
    Restricted,
}

impl SecurityLevel {
    /// [v0.5.0] 보안 수준 표시명 (영어 → 다국어 지원 시 msg!() 전환 가능)
    pub fn display(&self) -> &'static str {
        match self {
            SecurityLevel::Safe => "Safe",
            SecurityLevel::JailRequired => "Jail Required",
            SecurityLevel::Restricted => "Restricted",
        }
    }
}

/// [v0.4.0] 도구 파라미터 정의
#[derive(Debug, Clone)]
pub struct ToolParam {
    /// 파라미터 이름
    pub name: &'static str,
    /// 타입 (string, integer 등)
    pub param_type: &'static str,
    /// 필수 여부
    pub required: bool,
    /// 설명
    pub description: &'static str,
    /// 사용 예시
    pub example: &'static str,
}

/// [v0.4.0] 도구 정의
#[derive(Debug, Clone)]
pub struct ToolDef {
    /// 도구 표시명 (한국어)
    pub name: &'static str,
    /// 도구 식별자 (코드용)
    pub id: &'static str,
    /// 도구 설명
    pub description: &'static str,
    /// 파라미터 목록
    pub params: &'static [ToolParam],
    /// 제약 조건 (에이전트에게 전달)
    pub constraints: &'static str,
    /// 에러 발생 시 에이전트 대응 가이드
    pub error_guidance: &'static str,
    /// 보안 수준
    pub security_level: SecurityLevel,
}

// === 파라미터 상수 ===

const PARAM_PATH: ToolParam = ToolParam {
    name: "path",
    param_type: "string",
    required: true,
    description: "Relative path within workspace",
    example: "data/report.txt",
};

const PARAM_CONTENT: ToolParam = ToolParam {
    name: "content",
    param_type: "string",
    required: true,
    description: "Content to write to the file",
    example: "Analysis result: OK",
};

const PARAM_DIR: ToolParam = ToolParam {
    name: "dir",
    param_type: "string",
    required: true,
    description: "Relative directory path within workspace",
    example: "data",
};

const PARAM_MS: ToolParam = ToolParam {
    name: "ms",
    param_type: "integer",
    required: true,
    description: "Wait time in milliseconds (max 5000)",
    example: "1000",
};

const PARAM_MSG: ToolParam = ToolParam {
    name: "msg",
    param_type: "string",
    required: true,
    description: "Message to output",
    example: "Done!",
};

// === 내장 도구 5종 ===

/// [v0.4.0] 내장 도구 레지스트리
pub const BUILTIN_TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "File Read",
        id: "file_read",
        description: "Reads a file within workspace and returns its content. Text files only.",
        params: &[PARAM_PATH],
        constraints: "Path must be inside workspace/. \
                      Absolute paths and ../ traversal are blocked.",
        error_guidance: "If file not found, ask user to verify the path. \
                        On permission errors, try a different path.",
        security_level: SecurityLevel::JailRequired,
    },
    ToolDef {
        name: "File Write",
        id: "file_write",
        description:
            "Creates or overwrites a file within workspace. Intermediate dirs are auto-created.",
        params: &[PARAM_PATH, PARAM_CONTENT],
        constraints: "Path must be inside workspace/. \
                      Existing files are overwritten; suggest backup first. \
                      Paths containing ../ are blocked immediately.",
        error_guidance: "On write failure, check disk space or path validity. \
                        On security block, change to a workspace-internal path.",
        security_level: SecurityLevel::JailRequired,
    },
    ToolDef {
        name: "Directory List",
        id: "file_list",
        description: "Lists files/folders in a workspace directory.",
        params: &[PARAM_DIR],
        constraints: "Path must be inside workspace/. \
                      Empty directories return an empty array.",
        error_guidance: "If directory not found, ask user to verify path. \
                        Try file_list('.') first to see the full structure.",
        security_level: SecurityLevel::JailRequired,
    },
    ToolDef {
        name: "Sleep",
        id: "sleep",
        description: "Pauses execution for the specified milliseconds. Max 5000ms (5 seconds).",
        params: &[PARAM_MS],
        constraints: "Auto-clamped to 0~5000ms range. \
                      For longer waits, call multiple times.",
        error_guidance: "This tool never fails. Values over 5000ms are clamped to 5000ms.",
        security_level: SecurityLevel::Safe,
    },
    ToolDef {
        name: "Print",
        id: "print",
        description: "Writes a message to the output buffer. Included in script results.",
        params: &[PARAM_MSG],
        constraints: "Output accumulates in buffer and is included in script result.",
        error_guidance: "This tool never fails.",
        security_level: SecurityLevel::Safe,
    },
];

/// [v0.4.0] ID로 도구 정의를 찾는다
pub fn find_tool(id: &str) -> Option<&'static ToolDef> {
    BUILTIN_TOOLS.iter().find(|t| t.id == id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_tools_count() {
        assert_eq!(BUILTIN_TOOLS.len(), 5);
    }

    #[test]
    fn test_find_tool() {
        let tool = find_tool("file_read").unwrap();
        assert_eq!(tool.name, "File Read");
        assert_eq!(tool.security_level, SecurityLevel::JailRequired);
        assert!(!tool.params.is_empty());

        let safe = find_tool("print").unwrap();
        assert_eq!(safe.security_level, SecurityLevel::Safe);

        assert!(find_tool("nonexistent").is_none());
    }

    #[test]
    fn test_tool_params() {
        let tool = find_tool("file_write").unwrap();
        assert_eq!(tool.params.len(), 2);
        assert_eq!(tool.params[0].name, "path");
        assert_eq!(tool.params[1].name, "content");
        assert!(tool.params[0].required);
    }

    #[test]
    fn test_security_levels() {
        // 파일 I/O는 JailRequired
        assert_eq!(
            find_tool("file_read").unwrap().security_level,
            SecurityLevel::JailRequired
        );
        assert_eq!(
            find_tool("file_write").unwrap().security_level,
            SecurityLevel::JailRequired
        );
        assert_eq!(
            find_tool("file_list").unwrap().security_level,
            SecurityLevel::JailRequired
        );
        // print, sleep은 Safe
        assert_eq!(
            find_tool("print").unwrap().security_level,
            SecurityLevel::Safe
        );
        assert_eq!(
            find_tool("sleep").unwrap().security_level,
            SecurityLevel::Safe
        );
    }
}
