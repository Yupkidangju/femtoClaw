// femtoClaw — 진입점
// [v0.1.0] Step 1~5 통합: 샌드박스 초기화, TUI/Headless 분기,
// Graceful Shutdown (Ctrl+C → 트랜잭션 커밋 후 종료).
//
// 실행 흐름:
// 1. CLI 인자 파싱 (--headless)
// 2. 샌드박스 경로 해석 및 디렉토리 생성
// 3. 프로세스 락 획득 (중복 실행 방지)
// 4. Ctrl+C 시그널 핸들러 등록 (Graceful Shutdown)
// 5. TUI 모드: ratatui Amber Monochrome UI 실행
//    Headless 모드: 텔레그램 전용 봇 실행

mod config;
mod core;
mod db;
mod error;
mod sandbox;
mod security;
mod skills;
mod tools;
mod tui;

use error::FemtoResult;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

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

/// [v0.1.0] Graceful Shutdown을 위한 Ctrl+C 핸들러 등록.
/// 반환된 AtomicBool이 true가 되면 모든 스레드가 종료를 시작해야 한다.
fn setup_shutdown_handler() -> Arc<AtomicBool> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let flag = shutdown.clone();

    // Ctrl+C (SIGINT/SIGTERM) 핸들러
    ctrlc::set_handler(move || {
        eprintln!("\n[femtoClaw] 종료 신호 수신 — Graceful Shutdown 진행 중...");
        flag.store(true, Ordering::SeqCst);
    })
    .unwrap_or_else(|e| {
        eprintln!("[경고] Ctrl+C 핸들러 등록 실패: {}", e);
    });

    shutdown
}

/// [v0.1.0] 메인 초기화 → TUI/Headless 분기
fn run() -> FemtoResult<()> {
    let mode = parse_args();

    // 1. 샌드박스 초기화
    let paths = sandbox::SandboxPaths::resolve()?;
    sandbox::init_directories(&paths)?;

    // 2. 프로세스 락 획득
    let _lock = sandbox::ProcessLock::acquire(&paths.lock_file)?;

    // 3. Graceful Shutdown 핸들러
    let _shutdown_flag = setup_shutdown_handler();

    // 4. 모드별 분기
    match mode {
        RunMode::Tui => {
            let mut app = tui::app::App::new(paths);
            tui::run(&mut app)?;
        }
        RunMode::Headless => {
            run_headless(&paths, _shutdown_flag)?;
        }
    }

    Ok(())
}

/// [v0.1.0] 헤드리스 모드: TUI 없이 텔레그램 봇만 실행.
/// config.enc에서 설정 로드 → 텔레그램 봇 시작 → 종료 신호까지 대기.
fn run_headless(paths: &sandbox::SandboxPaths, shutdown_flag: Arc<AtomicBool>) -> FemtoResult<()> {
    eprintln!("┌──────────────────────────────────────────┐");
    eprintln!("│  femtoClaw v0.1.0-beta — Headless Mode   │");
    eprintln!("└──────────────────────────────────────────┘");

    // config.enc 존재 여부 확인
    if !config::config_exists(&paths.config_enc) {
        eprintln!("[오류] config.enc가 없습니다. 먼저 TUI 모드로 실행하여 설정을 완료하세요.");
        eprintln!("       $ femtoclaw  (TUI 모드)");
        return Ok(());
    }

    // 비밀번호 입력 (터미널에서)
    eprintln!("[*] 마스터 비밀번호를 입력하세요:");
    let mut password = String::new();
    std::io::stdin().read_line(&mut password).map_err(|e| {
        crate::error::FemtoError::ConfigIo(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("입력 실패: {}", e),
        ))
    })?;
    let password = password.trim();

    // config.enc 로드
    let app_config = config::load_config(password.as_bytes(), &paths.config_enc)?;

    // 텔레그램 토큰 확인
    let tg_token = match &app_config.telegram {
        Some(tg) if tg.verified => tg.bot_token.clone(),
        _ => {
            eprintln!("[오류] 텔레그램 설정이 없거나 미검증 상태입니다.");
            return Ok(());
        }
    };

    eprintln!("[*] 텔레그램 봇 시작 중...");

    let bot_shutdown = shutdown_flag.clone();
    let (event_rx, _cmd_tx, pin) = core::telegram::spawn_bot(tg_token, bot_shutdown);

    eprintln!("[✓] 봇 활성 — 페어링 PIN: {}", pin);
    eprintln!("[*] Ctrl+C로 종료합니다.");

    // 이벤트 루프: 종료 신호까지 대기하면서 봇 이벤트 처리
    loop {
        if shutdown_flag.load(Ordering::Relaxed) {
            eprintln!("[*] Graceful Shutdown 완료.");
            break;
        }

        // 봇 이벤트 수신 (100ms 타임아웃)
        match event_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(core::telegram::BotEvent::Paired(chat_id, username)) => {
                eprintln!("[✓] 페어링 성공: {} (chat_id: {})", username, chat_id);
            }
            Ok(core::telegram::BotEvent::MessageReceived(msg)) => {
                eprintln!("[→] 메시지 수신: {}", msg);
                // TODO: 에이전트 응답 로직 연결
            }
            Ok(core::telegram::BotEvent::Error(err)) => {
                eprintln!("[!] {}", err);
            }
            Ok(core::telegram::BotEvent::Shutdown) => {
                eprintln!("[*] 봇 종료됨.");
                break;
            }
            _ => {} // 타임아웃 또는 기타
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
