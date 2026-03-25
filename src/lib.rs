// femtoClaw — 라이브러리 진입점
// [v0.8.0] 통합 테스트(tests/)에서 내부 모듈에 접근하기 위한 lib.rs
//
// main.rs와 동일한 모듈 트리를 노출한다.
// 바이너리 진입점은 여전히 main.rs의 fn main()이다.

#[macro_use]
pub mod i18n;

pub mod config;
pub mod core;
pub mod db;
pub mod error;
pub mod sandbox;
pub mod security;
pub mod skills;
pub mod tools;
pub mod tui;
