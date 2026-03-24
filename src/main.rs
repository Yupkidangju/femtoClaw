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

// [v0.5.0] i18n 모듈은 msg!() 매크로를 전역에 노출하므로
// 다른 모든 mod 선언보다 먼저 위치해야 한다.
#[macro_use]
mod i18n;

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

/// [v0.5.0] CLI 인자 파싱 (--headless, --lang)
fn parse_args() -> RunMode {
    let args: Vec<String> = std::env::args().collect();

    // --lang <code> 오버라이드 (OS 감지보다 우선)
    for i in 0..args.len() {
        if args[i] == "--lang" {
            if let Some(code) = args.get(i + 1) {
                if let Some(lang) = i18n::Lang::from_code(code) {
                    i18n::set_lang(lang);
                }
            }
        }
    }

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
        eprintln!("\n[femtoClaw] {} ...", msg!("cli.graceful_shutdown"));
        flag.store(true, Ordering::SeqCst);
    })
    .unwrap_or_else(|e| {
        eprintln!("[!] Ctrl+C handler failed: {}", e);
    });

    shutdown
}

/// [v0.5.0] 메인 초기화 → i18n 감지 → TUI/Headless 분기
fn run() -> FemtoResult<()> {
    // OS 시스템 언어 자동 감지 (미지원 언어 → 영어 fallback)
    i18n::detect_and_set_lang();

    // --lang 인자가 있으면 detect 결과를 덮어쓴
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
    eprintln!("│  femtoClaw v0.4.0 — Headless Mode          │");
    eprintln!("└──────────────────────────────────────────┘");

    // config.enc 존재 여부 확인
    if !config::config_exists(&paths.config_enc) {
        eprintln!("[!] {}", msg!("cli.no_config"));
        eprintln!("    $ femtoclaw  (TUI)");
        return Ok(());
    }

    // 비밀번호 입력 (터미널에서)
    eprint!("[*] {}", msg!("cli.enter_pw"));
    let mut password = String::new();
    std::io::stdin().read_line(&mut password).map_err(|e| {
        crate::error::FemtoError::ConfigIo(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("stdin: {}", e),
        ))
    })?;
    let password = password.trim();

    // config.enc 로드
    let mut app_config = config::load_config(password.as_bytes(), &paths.config_enc)?;

    // 텔레그램 토큰 확인
    let tg_token = match &app_config.telegram {
        Some(tg) if tg.verified => tg.bot_token.clone(),
        _ => {
            eprintln!("[!] {}", msg!("cli.no_telegram"));
            return Ok(());
        }
    };

    eprintln!("[*] Telegram bot starting...");

    // [v0.4.0] 이전 페어링 chat_id 복원
    let saved_chat_id = app_config.telegram.as_ref().and_then(|tg| tg.chat_id);

    let bot_shutdown = shutdown_flag.clone();
    let (event_rx, _cmd_tx, pin) = core::telegram::spawn_bot(tg_token, bot_shutdown, saved_chat_id);

    eprintln!("[✓] Bot active — PIN: {}", pin);
    eprintln!("[*] Ctrl+C to quit.");

    // 이벤트 루프: 종료 신호까지 대기하면서 봇 이벤트 처리
    loop {
        if shutdown_flag.load(Ordering::Relaxed) {
            eprintln!("[*] {}", msg!("cli.graceful_shutdown"));
            break;
        }

        // 봇 이벤트 수신 (100ms 타임아웃)
        match event_rx.recv_timeout(std::time::Duration::from_millis(100)) {
            Ok(core::telegram::BotEvent::Paired(chat_id, username)) => {
                eprintln!("[✓] {}", msg!("cli.paired", username, chat_id));
                // [v0.5.0] 페어링 성공 시 chat_id를 config.enc에 영속화
                if let Some(ref mut tg) = app_config.telegram {
                    tg.chat_id = Some(chat_id);
                }
                if let Err(e) =
                    config::save_config(&app_config, password.as_bytes(), &paths.config_enc)
                {
                    eprintln!("[!] {}", msg!("cli.chat_save_fail", e));
                } else {
                    eprintln!("[✓] {}", msg!("cli.chat_saved"));
                }
            }
            Ok(core::telegram::BotEvent::MessageReceived(m)) => {
                eprintln!("[→] {}", msg!("cli.msg_received", m));
                // TODO: 에이전트 응답 로직 연결
            }
            Ok(core::telegram::BotEvent::Error(err)) => {
                eprintln!("[!] {}", err);
            }
            Ok(core::telegram::BotEvent::Shutdown) => {
                eprintln!("[*] {}", msg!("cli.bot_shutdown"));
                break;
            }
            _ => {} // 타임아웃 또는 기타
        }
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("[ERROR] {}", e);
        std::process::exit(1);
    }
}
