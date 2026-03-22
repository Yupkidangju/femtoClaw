// femtoClaw — 진입점
// [v0.1.0] Step 1+2: 샌드박스 초기화 후 TUI 또는 헤드리스 모드로 분기.
//
// 실행 흐름:
// 1. CLI 인자 파싱 (--headless)
// 2. 샌드박스 경로 해석 및 디렉토리 생성
// 3. 프로세스 락 획득 (중복 실행 방지)
// 4. TUI 모드: ratatui Amber Monochrome UI 실행
//    Headless 모드: 텔레그램 전용 (Step 4에서 구현)

mod config;
mod core;
mod db;
mod error;
mod sandbox;
mod security;
mod skills;
mod tui;

use error::FemtoResult;

/// [v0.1.0] 앱 실행 모드
#[derive(Debug, Clone, PartialEq)]
enum RunMode {
    Tui,
    Headless,
}

/// [v0.1.0] CLI 인자 파싱
fn parse_args() -> RunMode {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--headless") {
        RunMode::Headless
    } else {
        RunMode::Tui
    }
}

/// [v0.1.0] 메인 초기화 → TUI/Headless 분기
fn run() -> FemtoResult<()> {
    let mode = parse_args();

    // 1. 샌드박스 초기화
    let paths = sandbox::SandboxPaths::resolve()?;
    sandbox::init_directories(&paths)?;

    // 2. 프로세스 락 획득
    let _lock = sandbox::ProcessLock::acquire(&paths.lock_file)?;

    // 3. 모드별 분기
    match mode {
        RunMode::Tui => {
            let mut app = tui::app::App::new(paths);
            tui::run(&mut app)?;
        }
        RunMode::Headless => {
            eprintln!("[femtoClaw] 헤드리스 모드는 Step 4에서 구현됩니다.");
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[오류] {}", e);
        std::process::exit(1);
    }
}
