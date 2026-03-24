// femtoClaw — 코어 엔진 모듈
// [v0.6.0] Agent Runtime: Bootstrap + Persona + Chat Loop + LLM + Telegram
//
// 구조:
//   persona.rs       — agent.toml/user.toml 파서 (페르소나·사용자 프로필)
//   bootstrap.rs     — 첫 실행 감지 → 파일 시드 → 초기화
//   agent.rs         — LLM 대화 클라이언트 (/chat/completions)
//   telegram.rs      — teloxide 봇 (PIN 페어링, 메시지 라우팅)
//   agent_manager.rs — 멀티 에이전트 경로 격리 및 관리

pub mod agent;
pub mod agent_manager;
pub mod bootstrap;
pub mod persona;
pub mod telegram;
