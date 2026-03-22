// femtoClaw — 코어 엔진 모듈
// [v0.1.0] Step 4: LLM 에이전트 + 텔레그램 통신 엔진
//
// 구조:
//   agent.rs     — LLM 대화 클라이언트 (/chat/completions)
//   telegram.rs  — teloxide 봇 (PIN 페어링, 메시지 라우팅)

pub mod agent;
pub mod telegram;
