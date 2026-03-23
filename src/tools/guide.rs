// femtoClaw — Jailing 가이드 시스템
// [v0.4.0] Step 9d: 사용자가 Jailing을 몰라도 좋은 UX를 제공한다.
//
// 설계 원칙:
//   - "안 됩니다"가 아닌 "이렇게 하면 됩니다"로 안내
//   - 상황별 맞춤 메시지 (경로 탈출, 명령어 차단, 첫 안내 등)
//   - 에이전트가 대화에서 자연스럽게 사용할 수 있는 문구

/// [v0.4.0] Jailing 가이드 — 사용자 친화적 안내 메시지 생성기
pub struct JailingGuide;

impl JailingGuide {
    /// [v0.4.0] 첫 대화 시 workspace 개념을 소개하는 메시지
    pub fn welcome_message() -> &'static str {
        "안녕하세요! 저는 femtoClaw 에이전트입니다. 🛡️\n\
         보안을 위해 저는 전용 workspace 안에서만 파일 작업을 할 수 있습니다.\n\
         파일을 분석하고 싶으시면 workspace/data/에 넣어주시면 바로 시작할게요!"
    }

    /// [v0.4.0] Jailing 차단 시 친절한 설명 + 대안 안내
    pub fn explain_block(detail: &str) -> String {
        // 경로 탈출 시도 감지
        if detail.contains("경로 탈출") || detail.contains("밖") {
            return format!(
                "🔒 요청하신 경로는 workspace 밖이라 접근할 수 없습니다.\n\n\
                 이것은 여러분의 시스템을 보호하기 위한 안전장치입니다.\n\n\
                 💡 해결 방법:\n\
                 • 해당 파일을 workspace/data/ 폴더에 복사해주시면 바로 읽을 수 있어요!\n\
                 • 또는 file_list('.')으로 현재 workspace 내용을 확인해보세요.\n\n\
                 상세: {}",
                detail
            );
        }

        // ../ 디렉토리 순회 감지
        if detail.contains("..") || detail.contains("순회") {
            return format!(
                "🔒 '../'를 사용한 상위 폴더 접근은 보안 차단됩니다.\n\n\
                 workspace 내부의 파일만 사용할 수 있어요.\n\n\
                 💡 workspace/data/ 에 파일을 넣어주시면 분석해드릴게요!\n\n\
                 상세: {}",
                detail
            );
        }

        // 명령어 차단 감지
        if detail.contains("금지 명령어") || detail.contains("BLOCKED") {
            return format!(
                "🔒 해당 명령어는 시스템 보안을 위해 사용할 수 없습니다.\n\n\
                 femtoClaw는 파괴적 시스템 명령을 사전에 차단합니다.\n\
                 안전한 파일 도구(file_read/write/list)를 사용해주세요.\n\n\
                 상세: {}",
                detail
            );
        }

        // 기본 안내
        format!(
            "🔒 보안 정책으로 해당 작업이 차단되었습니다.\n\n\
             저는 workspace/ 내에서만 안전하게 작업할 수 있습니다.\n\
             이것은 여러분의 시스템을 보호하기 위한 안전장치입니다.\n\n\
             💡 파일을 workspace/data/에 넣어주시면 분석해드릴게요!\n\n\
             상세: {}",
            detail
        )
    }

    /// [v0.4.0] workspace 구조를 사용자에게 설명하는 메시지
    pub fn explain_workspace() -> &'static str {
        "📁 workspace 구조 안내\n\n\
         femtoClaw의 workspace는 에이전트의 전용 작업 공간입니다:\n\n\
         workspace/\n\
         ├── data/     ← 파일을 여기에 넣어주세요\n\
         └── temp/     ← 임시 파일 (자동 정리)\n\n\
         • data/: 분석할 파일, 보고서, 데이터를 넣는 곳\n\
         • temp/: 에이전트가 임시로 사용하는 공간\n\n\
         data/ 폴더에 파일을 넣으시면 바로 읽고 분석할 수 있습니다!"
    }

    /// [v0.4.0] 에러 유형에 따른 추가 도움 메시지
    pub fn help_for_error(error_type: &str) -> String {
        match error_type {
            "FileNotFound" => "💡 파일을 찾을 수 없을 때:\n\
                 1. file_list('.')으로 현재 파일 목록 확인\n\
                 2. file_list('data/')으로 data 폴더 확인\n\
                 3. 파일명에 오타가 없는지 확인\n\
                 4. 파일을 workspace/data/에 복사한 후 재시도"
                .to_string(),
            "PermissionDenied" => "💡 권한 문제가 발생했을 때:\n\
                 1. 파일이 다른 프로그램에서 사용 중인지 확인\n\
                 2. 읽기 전용 파일이 아닌지 확인\n\
                 3. workspace/temp/에 복사본을 만들어 시도"
                .to_string(),
            _ => "💡 문제가 지속되면:\n\
                 1. 에이전트를 재시작해보세요\n\
                 2. workspace 폴더의 상태를 확인해주세요\n\
                 3. 디스크 공간이 충분한지 확인해주세요"
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
        assert!(msg.contains("보안"));
        assert!(msg.contains("data/"));
    }

    #[test]
    fn test_explain_block_path_escape() {
        let msg = JailingGuide::explain_block("경로 탈출 시도 — /etc/passwd");
        assert!(msg.contains("workspace 밖"));
        assert!(msg.contains("복사"));
        assert!(msg.contains("해결 방법"));
    }

    #[test]
    fn test_explain_block_traversal() {
        let msg = JailingGuide::explain_block("디렉토리 순회(../) 금지");
        assert!(msg.contains("../"));
        assert!(msg.contains("파일을 넣어주시면"));
    }

    #[test]
    fn test_explain_block_command() {
        let msg = JailingGuide::explain_block("금지 명령어 — rm -rf");
        assert!(msg.contains("명령어"));
        assert!(msg.contains("안전한"));
    }

    #[test]
    fn test_explain_workspace() {
        let msg = JailingGuide::explain_workspace();
        assert!(msg.contains("data/"));
        assert!(msg.contains("temp/"));
        assert!(msg.contains("전용 작업 공간"));
    }

    #[test]
    fn test_help_for_error() {
        let msg = JailingGuide::help_for_error("FileNotFound");
        assert!(msg.contains("file_list"));

        let msg = JailingGuide::help_for_error("PermissionDenied");
        assert!(msg.contains("권한"));

        let msg = JailingGuide::help_for_error("Unknown");
        assert!(msg.contains("재시작"));
    }
}
