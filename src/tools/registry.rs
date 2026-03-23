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
    /// 한국어 표시명
    pub fn display(&self) -> &'static str {
        match self {
            SecurityLevel::Safe => "안전",
            SecurityLevel::JailRequired => "Jail 검증",
            SecurityLevel::Restricted => "제한됨",
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
    description: "workspace 내 상대 경로",
    example: "data/report.txt",
};

const PARAM_CONTENT: ToolParam = ToolParam {
    name: "content",
    param_type: "string",
    required: true,
    description: "파일에 쓸 내용",
    example: "분석 결과: 정상",
};

const PARAM_DIR: ToolParam = ToolParam {
    name: "dir",
    param_type: "string",
    required: true,
    description: "workspace 내 디렉토리 상대 경로",
    example: "data",
};

const PARAM_MS: ToolParam = ToolParam {
    name: "ms",
    param_type: "integer",
    required: true,
    description: "대기 시간 (밀리초, 최대 5000)",
    example: "1000",
};

const PARAM_MSG: ToolParam = ToolParam {
    name: "msg",
    param_type: "string",
    required: true,
    description: "출력할 메시지",
    example: "처리 완료!",
};

// === 내장 도구 5종 ===

/// [v0.4.0] 내장 도구 레지스트리
pub const BUILTIN_TOOLS: &[ToolDef] = &[
    ToolDef {
        name: "파일 읽기",
        id: "file_read",
        description: "workspace 내 파일을 읽어 내용을 반환합니다. 텍스트 파일만 지원합니다.",
        params: &[PARAM_PATH],
        constraints: "경로는 반드시 workspace/ 내부여야 합니다. \
                      절대 경로나 ../를 사용한 상위 디렉토리 접근은 차단됩니다.",
        error_guidance: "파일이 없으면 사용자에게 파일 경로를 확인해달라고 요청하세요. \
                        권한 문제가 발생하면 다른 경로를 시도하세요.",
        security_level: SecurityLevel::JailRequired,
    },
    ToolDef {
        name: "파일 쓰기",
        id: "file_write",
        description:
            "workspace 내에 파일을 생성하거나 덮어씁니다. 중간 디렉토리는 자동 생성됩니다.",
        params: &[PARAM_PATH, PARAM_CONTENT],
        constraints: "경로는 반드시 workspace/ 내부여야 합니다. \
                      기존 파일은 덮어쓰여지므로, 중요한 파일은 먼저 백업을 제안하세요. \
                      ../를 포함한 경로는 즉시 차단됩니다.",
        error_guidance: "쓰기 실패 시 디스크 용량이나 경로 유효성을 확인하세요. \
                        보안 차단이 발생하면 workspace 내부 경로로 변경하세요.",
        security_level: SecurityLevel::JailRequired,
    },
    ToolDef {
        name: "디렉토리 목록",
        id: "file_list",
        description: "workspace 내 디렉토리의 파일/폴더 목록을 반환합니다.",
        params: &[PARAM_DIR],
        constraints: "경로는 반드시 workspace/ 내부여야 합니다. \
                      비어있는 디렉토리는 빈 배열을 반환합니다.",
        error_guidance: "디렉토리가 없으면 사용자에게 경로를 확인해달라고 요청하세요. \
                        먼저 file_list('.')으로 전체 구조를 파악하는 것이 좋습니다.",
        security_level: SecurityLevel::JailRequired,
    },
    ToolDef {
        name: "대기",
        id: "sleep",
        description: "지정한 시간(밀리초) 동안 실행을 멈춥니다. 최대 5000ms(5초)까지 허용됩니다.",
        params: &[PARAM_MS],
        constraints: "0~5000ms 범위로 자동 클램핑됩니다. \
                      긴 대기가 필요하면 여러 번 나눠서 호출하세요.",
        error_guidance:
            "이 도구는 실패하지 않습니다. 5000ms를 초과하면 자동으로 5000ms로 제한됩니다.",
        security_level: SecurityLevel::Safe,
    },
    ToolDef {
        name: "로그 출력",
        id: "print",
        description: "메시지를 출력 버퍼에 기록합니다. 스크립트 실행 결과에 포함됩니다.",
        params: &[PARAM_MSG],
        constraints: "출력은 버퍼에 누적되며, 스크립트 종료 후 결과에 포함됩니다.",
        error_guidance: "이 도구는 실패하지 않습니다.",
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
        assert_eq!(tool.name, "파일 읽기");
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
