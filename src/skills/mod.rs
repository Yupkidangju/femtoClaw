// femtoClaw — 스킬 시스템 모듈
// [v0.2.0] Step 5/6: 정적(TOML) + 동적(Rhai) 하이브리드 스킬 시스템.
//
// 스킬 정의: TOML 파일 (정적) + .rhai 파일 (동적)
// 로딩: 앱 시작 시 skills/ 디렉토리 스캔 → 일괄 로드
// 생성: 대화를 통해 skills/user/에 자동 저장

pub mod loader;
pub mod rhai_engine;

pub use loader::{load_skills_from_dir, save_skill, Skill, SkillAction, SkillType};
pub use rhai_engine::RhaiEngine;
