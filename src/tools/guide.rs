// femtoClaw — Jailing 가이드 시스템
// [v0.4.0] Step 9d: 사용자가 Jailing을 몰라도 좋은 UX를 제공한다.
// [v0.5.0] i18n: 영어 기본값. 향후 msg!() 전환으로 다국어 지원 가능.
//
// 설계 원칙:
//   - "안 됩니다"가 아닌 "이렇게 하면 됩니다"로 안내
//   - 상황별 맞춤 메시지 (경로 탈출, 명령어 차단, 첫 안내 등)
//   - 에이전트가 대화에서 자연스럽게 사용할 수 있는 문구

/// [v0.4.0] Jailing 가이드 — 사용자 친화적 안내 메시지 생성기
pub struct JailingGuide;

impl JailingGuide {
    /// [v0.5.0] 첫 대화 시 workspace 개념을 소개하는 메시지
    pub fn welcome_message() -> &'static str {
        "Hello! I'm the femtoClaw agent. 🛡️\n\
         For security, I can only work with files inside the dedicated workspace.\n\
         Place files in workspace/data/ and I'll start right away!"
    }

    /// [v0.5.0] Jailing 차단 시 친절한 설명 + 대안 안내
    pub fn explain_block(detail: &str) -> String {
        // 경로 탈출 시도 감지
        if detail.contains("path escape")
            || detail.contains("outside")
            || detail.contains("Absolute")
        {
            return format!(
                "🔒 The requested path is outside workspace and cannot be accessed.\n\n\
                 This is a safety measure to protect your system.\n\n\
                 💡 Solution:\n\
                 • Copy the file into workspace/data/ and I can read it right away!\n\
                 • Or use file_list('.') to see what's in workspace.\n\n\
                 Detail: {}",
                detail
            );
        }

        // ../ 디렉토리 순회 감지
        if detail.contains("..") || detail.contains("traversal") {
            return format!(
                "🔒 Using '../' to access parent directories is blocked for security.\n\n\
                 Only files inside workspace can be used.\n\n\
                 💡 Place files in workspace/data/ and I can analyze them!\n\n\
                 Detail: {}",
                detail
            );
        }

        // 명령어 차단 감지
        if detail.contains("blocked command") || detail.contains("BLOCKED") {
            return format!(
                "🔒 This command is blocked for system security.\n\n\
                 femtoClaw blocks destructive system commands preemptively.\n\
                 Please use safe file tools (file_read/write/list).\n\n\
                 Detail: {}",
                detail
            );
        }

        // 기본 안내
        format!(
            "🔒 This action was blocked by security policy.\n\n\
             I can only work safely within workspace/.\n\
             This is a safety measure to protect your system.\n\n\
             💡 Place files in workspace/data/ and I can analyze them!\n\n\
             Detail: {}",
            detail
        )
    }

    /// [v0.5.0] workspace 구조를 사용자에게 설명하는 메시지
    pub fn explain_workspace() -> &'static str {
        "📁 Workspace Structure\n\n\
         The femtoClaw workspace is the agent's dedicated working area:\n\n\
         workspace/\n\
         ├── data/     ← Place your files here\n\
         └── temp/     ← Temporary files (auto-cleaned)\n\n\
         • data/: For files, reports, and data to analyze\n\
         • temp/: Used by the agent temporarily\n\n\
         Place files in data/ and I can read and analyze them!"
    }

    /// [v0.5.0] 에러 유형에 따른 추가 도움 메시지
    pub fn help_for_error(error_type: &str) -> String {
        match error_type {
            "FileNotFound" => "💡 When a file can't be found:\n\
                 1. Check current files with file_list('.')\n\
                 2. Check data folder with file_list('data/')\n\
                 3. Verify the filename for typos\n\
                 4. Copy the file to workspace/data/ and retry"
                .to_string(),
            "PermissionDenied" => "💡 When permission is denied:\n\
                 1. Check if the file is in use by another program\n\
                 2. Check if the file is read-only\n\
                 3. Try making a copy in workspace/temp/"
                .to_string(),
            _ => "💡 If the problem persists:\n\
                 1. Try restarting the agent\n\
                 2. Check workspace folder status\n\
                 3. Verify sufficient disk space"
                .to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_welcome_message() {
        let msg = JailingGuide::welcome_message();
        assert!(msg.contains("workspace"));
        assert!(msg.contains("security"));
        assert!(msg.contains("data/"));
    }

    #[test]
    fn test_explain_block_path_escape() {
        let msg = JailingGuide::explain_block("Absolute path '/etc/passwd' not allowed");
        assert!(msg.contains("outside workspace"));
        assert!(msg.contains("Copy"));
        assert!(msg.contains("Solution"));
    }

    #[test]
    fn test_explain_block_traversal() {
        let msg = JailingGuide::explain_block("Directory traversal (../) forbidden");
        assert!(msg.contains("../"));
        assert!(msg.contains("Place files"));
    }

    #[test]
    fn test_explain_block_command() {
        let msg = JailingGuide::explain_block("BLOCKED: rm -rf not allowed");
        assert!(msg.contains("command"));
        assert!(msg.contains("safe file tools"));
    }

    #[test]
    fn test_explain_workspace() {
        let msg = JailingGuide::explain_workspace();
        assert!(msg.contains("data/"));
        assert!(msg.contains("temp/"));
        assert!(msg.contains("dedicated working area"));
    }

    #[test]
    fn test_help_for_error() {
        let msg = JailingGuide::help_for_error("FileNotFound");
        assert!(msg.contains("file_list"));

        let msg = JailingGuide::help_for_error("PermissionDenied");
        assert!(msg.contains("permission"));

        let msg = JailingGuide::help_for_error("Unknown");
        assert!(msg.contains("restarting"));
    }
}
