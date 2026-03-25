// femtoClaw — 코어 엔진 모듈
// [v0.6.0] Agent Runtime: Bootstrap + Persona + Chat Loop + LLM + Telegram
// [v0.8.0] Schedule: 내장 스케줄러 + OS 네이티브 예약 등록
//
// 구조:
//   persona.rs       — agent.toml/user.toml 파서 (페르소나·사용자 프로필)
//   bootstrap.rs     — 첫 실행 감지 → 파일 시드 → 초기화
//   context.rs       — 컨텍스트 윈도우 조립 + tiktoken 토큰 카운터
//   tool_protocol.rs — OpenAI Function Calling 프로토콜 변환
//   chat_loop.rs     — 대화 루프 본체 (handle_message 단일 함수)
//   agent.rs         — LLM 대화 클라이언트 (/chat/completions + tools)
//   telegram.rs      — teloxide 봇 (PIN 페어링, 메시지 라우팅)
//   agent_manager.rs — 멀티 에이전트 경로 격리 및 관리
//   schedule.rs      — [v0.8.0] 내장 스케줄러 (cron 파서 + 타이머 루프)
//   install.rs       — [v0.8.0] OS 네이티브 예약 등록/해제

pub mod agent;
pub mod agent_manager;
pub mod bootstrap;
pub mod chat_loop;
pub mod context;
pub mod install;
pub mod persona;
pub mod schedule;
pub mod telegram;
pub mod tool_protocol;
