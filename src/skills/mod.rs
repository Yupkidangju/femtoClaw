// femtoClaw — 스킬 시스템 모듈
// [v0.1.0] Step 5: 정적 파일 기반 스킬 로더.
//
// 스킬 정의: TOML 파일 (skills/core/, skills/user/)
// 로딩: 앱 시작 시 디렉토리 스캔 → 일괄 로드
// 생성: 대화를 통해 skills/user/에 자동 저장

pub mod loader;

pub use loader::{load_skills_from_dir, save_skill, Skill, SkillAction};
