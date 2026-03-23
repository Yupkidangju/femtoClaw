// femtoClaw — 도구 하네스 모듈
// [v0.4.0] Step 9: Tool Registry + System Prompt + Safe Executor + Jailing Guide
//
// 구조:
//   registry.rs     — ToolDef, ToolParam, SecurityLevel + 내장 도구 5종 정의
//   prompt.rs       — LLM 시스템 프롬프트 빌더 (도구 명세 + Jailing + 에러 규칙)
//   executor.rs     — 안전 실행 레이어 (검증 → 실행 → 에러 복구)
//   guide.rs        — Jailing 가이드 (사용자 친화적 안내 메시지 생성)

pub mod executor;
pub mod guide;
pub mod prompt;
pub mod registry;

pub use executor::{ToolError, ToolExecutor, ToolResult};
pub use guide::JailingGuide;
pub use prompt::build_system_prompt;
pub use registry::{SecurityLevel, ToolDef, ToolParam, BUILTIN_TOOLS};
