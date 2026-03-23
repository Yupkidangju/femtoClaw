// femtoClaw — 코어 엔진 모듈
// [v0.3.0] Step 4/8: LLM 에이전트 + 텔레그램 통신 + 멀티 에이전트 매니저
//
// 구조:
//   agent.rs         — LLM 대화 클라이언트 (/chat/completions)
//   telegram.rs      — teloxide 봇 (PIN 페어링, 메시지 라우팅)
//   agent_manager.rs — 멀티 에이전트 경로 격리 및 관리

pub mod agent;
pub mod agent_manager;
pub mod telegram;
