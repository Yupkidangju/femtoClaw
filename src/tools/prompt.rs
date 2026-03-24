// femtoClaw — 시스템 프롬프트 빌더
// [v0.4.0] Step 9b: LLM에 보내는 system message에 도구 명세 + Jailing 개념 + 에러 규칙을 자동 주입한다.
// [v0.5.0] i18n: LLM 프롬프트는 영어 기본값. 에이전트가 사용자 언어로 응답.
//
// 설계 원칙:
//   - BUILTIN_TOOLS에서 도구 정보를 자동 추출하여 프롬프트 생성
//   - Jailing 경계를 자연어로 설명 → LLM이 사용자에게 친절히 안내 가능
//   - 에러 발생 시 대응 규칙까지 프롬프트에 포함
//   - 에이전트 이름과 역할을 동적으로 반영

use super::registry::BUILTIN_TOOLS;

/// [v0.5.0] Jailing 안내 블록 — LLM이 사용자에게 Jailing을 설명할 때 쓸 정보
const JAILING_SECTION: &str = "\
## Security Environment (Jailing)

I can only work within the **workspace/** directory for security.
This is a safety measure to protect the user's system.

### What is workspace?
- The agent's dedicated working area.
- Path: `~/.femtoclaw/workspace/` (or per-agent isolated path)
- You can read, write files and browse directories within it.

### What you cannot do
- Access files outside workspace (e.g., /etc/, C:\\\\Windows\\\\, user home)
- Use ../ to traverse to parent directories
- Execute system commands (rm -rf, format, sudo, etc.)

### When a user requests external files
Kindly suggest: \"Place the file in workspace/data/ and I can analyze it right away!\"
Never just say \"not allowed\" — always explain **how they can do it**.
";

/// [v0.5.0] 에러 처리 규칙 블록
const ERROR_RULES_SECTION: &str = "\
## Tool Error Handling Rules

1. **If a tool fails**: Clearly explain what went wrong and suggest alternatives.
2. **If the same tool fails 3+ times**: Stop retrying and ask the user for help.
3. **If security blocks occur**: Explain it's a security policy and suggest workspace-internal paths.
4. **If a file is not found**: Check directory structure with file_list() first, then retry.
5. **If no tool is needed**: Don't call tools. Answer conversational questions without tools.
";

/// [v0.5.0] 시스템 프롬프트를 생성한다.
///
/// LLM API 호출 시 messages[0]에 system role로 주입한다.
/// agent_name: 에이전트 이름 (예: "Alpha")
pub fn build_system_prompt(agent_name: &str) -> String {
    let mut prompt = String::with_capacity(4096);

    // 에이전트 정체성
    prompt.push_str(&format!(
        "# femtoClaw Agent: {}\n\n\
         You are \"{}\", an AI agent on the femtoClaw TUI platform.\n\
         Answer user questions and perform file tasks using tools.\n\
         Respond helpfully and accurately in the user's language.\n\n",
        agent_name, agent_name
    ));

    // 도구 명세
    prompt.push_str("## Available Tools\n\n");
    for tool in BUILTIN_TOOLS {
        prompt.push_str(&format!("### {} (`{}`)\n", tool.name, tool.id));
        prompt.push_str(&format!("{}\n\n", tool.description));

        // 파라미터
        if !tool.params.is_empty() {
            prompt.push_str("**Parameters:**\n");
            for p in tool.params {
                let req = if p.required { "required" } else { "optional" };
                prompt.push_str(&format!(
                    "- `{}` ({}, {}): {} (e.g., `{}`)\n",
                    p.name, p.param_type, req, p.description, p.example
                ));
            }
            prompt.push('\n');
        }

        // 제약
        prompt.push_str(&format!("**Constraints:** {}\n\n", tool.constraints));

        // 에러 가이드
        prompt.push_str(&format!("**On error:** {}\n\n", tool.error_guidance));

        prompt.push_str("---\n\n");
    }

    // Jailing 설명
    prompt.push_str(JAILING_SECTION);
    prompt.push('\n');

    // 에러 규칙
    prompt.push_str(ERROR_RULES_SECTION);

    prompt
}

/// [v0.5.0] 도구 목록만 간략히 반환 (헬프 표시용)
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
        assert!(prompt.contains("security"));
        // 에러 규칙 포함
        assert!(prompt.contains("3+ times"));
        // 적절한 길이 (최소 1000자 이상)
        assert!(prompt.len() > 1000);
    }

    #[test]
    fn test_tool_summary() {
        let summary = tool_summary();
        assert!(summary.contains("File Read"));
        assert!(summary.contains("File Write"));
        assert!(summary.contains("Jail Required"));
        assert!(summary.contains("Safe"));
    }

    #[test]
    fn test_prompt_contains_jailing_guide() {
        let prompt = build_system_prompt("Beta");
        // 친절한 안내 문구 포함
        assert!(prompt.contains("Place the file"));
        assert!(prompt.contains("how they can do it"));
    }
}
