// femtoClaw — 시스템 프롬프트 빌더
// [v0.4.0] Step 9b: LLM에 보내는 system message에 도구 명세 + Jailing 개념 + 에러 규칙을 자동 주입한다.
//
// 설계 원칙:
//   - BUILTIN_TOOLS에서 도구 정보를 자동 추출하여 프롬프트 생성
//   - Jailing 경계를 자연어로 설명 → LLM이 사용자에게 친절히 안내 가능
//   - 에러 발생 시 대응 규칙까지 프롬프트에 포함
//   - 에이전트 이름과 역할을 동적으로 반영

use super::registry::BUILTIN_TOOLS;

/// [v0.4.0] Jailing 안내 블록 — LLM이 사용자에게 Jailing을 설명할 때 쓸 정보
const JAILING_SECTION: &str = "\
## 보안 환경 (Jailing)

나는 보안을 위해 **workspace/** 디렉토리 내부에서만 작업할 수 있습니다.
이것은 사용자의 시스템을 보호하기 위한 안전장치입니다.

### workspace란?
- 에이전트의 전용 작업 공간입니다.
- 경로: `~/.femtoclaw/workspace/` (또는 에이전트별 격리 경로)
- 이 안에서 파일을 읽고, 쓰고, 디렉토리를 탐색할 수 있습니다.

### 할 수 없는 것
- workspace 밖의 파일 접근 (예: /etc/, C:\\Windows\\, 사용자 홈 디렉토리)
- ../를 사용한 상위 디렉토리 이동
- 시스템 명령어 실행 (rm -rf, format, sudo 등)

### 사용자가 외부 파일을 요청하면
\"workspace/data/ 에 파일을 복사해주시면 분석해드릴게요!\" 처럼 친절하게 대안을 제시합니다.
절대로 \"안 됩니다\"로만 끝내지 말고, **어떻게 하면 되는지** 알려주세요.
";

/// [v0.4.0] 에러 처리 규칙 블록
const ERROR_RULES_SECTION: &str = "\
## 도구 사용 시 에러 처리 규칙

1. **도구가 실패하면**: 사용자에게 무엇이 잘못되었는지 명확히 설명하고, 대안을 제시하세요.
2. **같은 도구를 3회 이상 연속 실패하면**: 중단하고 사용자에게 도움을 요청하세요.
3. **보안 차단이 발생하면**: \"이것은 보안 정책으로 제한됩니다\"라고 설명하고, workspace 내부 경로를 제안하세요.
4. **파일이 없으면**: 먼저 file_list()로 디렉토리 구조를 확인한 후 재시도하세요.
5. **도구를 사용할 필요가 없으면**: 도구를 호출하지 마세요. 대화만으로 해결 가능한 질문에는 도구 없이 답하세요.
";

/// [v0.4.0] 시스템 프롬프트를 생성한다.
///
/// LLM API 호출 시 messages[0]에 system role로 주입한다.
/// agent_name: 에이전트 이름 (예: "Alpha")
pub fn build_system_prompt(agent_name: &str) -> String {
    let mut prompt = String::with_capacity(4096);

    // 에이전트 정체성
    prompt.push_str(&format!(
        "# femtoClaw 에이전트: {}\n\n\
         당신은 femtoClaw TUI 플랫폼의 AI 에이전트 \"{}\"입니다.\n\
         사용자의 질문에 답하고, 도구를 활용하여 파일 작업을 수행합니다.\n\
         한국어로 친절하고 정확하게 답변합니다.\n\n",
        agent_name, agent_name
    ));

    // 도구 명세
    prompt.push_str("## 사용 가능한 도구\n\n");
    for tool in BUILTIN_TOOLS {
        prompt.push_str(&format!("### {} (`{}`)\n", tool.name, tool.id));
        prompt.push_str(&format!("{}\n\n", tool.description));

        // 파라미터
        if !tool.params.is_empty() {
            prompt.push_str("**파라미터:**\n");
            for p in tool.params {
                let req = if p.required { "필수" } else { "선택" };
                prompt.push_str(&format!(
                    "- `{}` ({}, {}): {} (예: `{}`)\n",
                    p.name, p.param_type, req, p.description, p.example
                ));
            }
            prompt.push('\n');
        }

        // 제약
        prompt.push_str(&format!("**제약:** {}\n\n", tool.constraints));

        // 에러 가이드
        prompt.push_str(&format!("**에러 시:** {}\n\n", tool.error_guidance));

        prompt.push_str("---\n\n");
    }

    // Jailing 설명
    prompt.push_str(JAILING_SECTION);
    prompt.push('\n');

    // 에러 규칙
    prompt.push_str(ERROR_RULES_SECTION);

    prompt
}

/// [v0.4.0] 도구 목록만 간략히 반환 (헬프 표시용)
pub fn tool_summary() -> String {
    let mut summary = String::new();
    for tool in BUILTIN_TOOLS {
        let sec = tool.security_level.display();
        summary.push_str(&format!(
            " • {} ({}) — {} [{}]\n",
            tool.name, tool.id, tool.description, sec
        ));
    }
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_system_prompt() {
        let prompt = build_system_prompt("Alpha");

        // 에이전트 이름 포함
        assert!(prompt.contains("Alpha"));
        // 도구 이름 포함
        assert!(prompt.contains("file_read"));
        assert!(prompt.contains("file_write"));
        assert!(prompt.contains("file_list"));
        assert!(prompt.contains("sleep"));
        assert!(prompt.contains("print"));
        // Jailing 설명 포함
        assert!(prompt.contains("workspace"));
        assert!(prompt.contains("보안"));
        // 에러 규칙 포함
        assert!(prompt.contains("3회 이상"));
        // 적절한 길이 (최소 1000자 이상)
        assert!(prompt.len() > 1000);
    }

    #[test]
    fn test_tool_summary() {
        let summary = tool_summary();
        assert!(summary.contains("파일 읽기"));
        assert!(summary.contains("파일 쓰기"));
        assert!(summary.contains("Jail 검증"));
        assert!(summary.contains("안전"));
    }

    #[test]
    fn test_prompt_contains_jailing_guide() {
        let prompt = build_system_prompt("Beta");
        // 친절한 안내 문구 포함
        assert!(prompt.contains("복사해주시면"));
        assert!(prompt.contains("어떻게 하면 되는지"));
    }
}
