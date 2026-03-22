// femtoClaw — TUI 모듈 (터미널 관리 및 메인 루프)
// [v0.1.0] Step 2: crossterm 기반 터미널 설정/복원 및 이벤트 루프.
// ratatui의 표준 패턴을 따르며, 패닉 시에도 터미널 상태를 안전하게 복원한다.

pub mod app;
pub mod theme;

use std::io::{self, stdout};
use std::time::Duration;

use crossterm::event::{self, Event};
use crossterm::execute;
use crossterm::terminal::{
    disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen,
};
use ratatui::backend::CrosstermBackend;
use ratatui::Terminal;

use crate::error::FemtoResult;
use app::App;

/// [v0.1.0] TUI 앱을 실행한다.
/// 터미널을 Raw 모드로 전환하고, 앱 루프를 실행한 뒤, 정상/비정상 종료 모두에서
/// 터미널 상태를 원래대로 복원한다.
pub fn run(app: &mut App) -> FemtoResult<()> {
    // 터미널 설정: Raw 모드 + 대체 화면 활성화
    enable_raw_mode().map_err(|e| crate::error::FemtoError::SandboxCreation(e))?;
    let mut stdout = stdout();
    execute!(stdout, EnterAlternateScreen)
        .map_err(|e| crate::error::FemtoError::SandboxCreation(e))?;

    let backend = CrosstermBackend::new(stdout);
    let mut terminal =
        Terminal::new(backend).map_err(|e| crate::error::FemtoError::SandboxCreation(e))?;

    // 패닉 훅 설치: 패닉 시에도 터미널 복원
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let _ = disable_raw_mode();
        let _ = execute!(io::stdout(), LeaveAlternateScreen);
        original_hook(info);
    }));

    // 메인 이벤트 루프 실행
    let result = main_loop(&mut terminal, app);

    // 터미널 복원
    disable_raw_mode().map_err(|e| crate::error::FemtoError::SandboxCreation(e))?;
    execute!(io::stdout(), LeaveAlternateScreen)
        .map_err(|e| crate::error::FemtoError::SandboxCreation(e))?;

    result
}

/// [v0.1.0] 메인 이벤트 루프.
/// 100ms 간격으로 이벤트를 폴링하며, 키 이벤트를 App에 전달한다.
/// App이 종료 상태가 되면 루프를 빠져나간다.
fn main_loop(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    app: &mut App,
) -> FemtoResult<()> {
    loop {
        // 화면 렌더링
        terminal
            .draw(|frame| app.render(frame))
            .map_err(|e| crate::error::FemtoError::SandboxCreation(e))?;

        // 이벤트 폴링 (100ms 타임아웃 → 비동기 결과 확인용 tick 보장)
        if event::poll(Duration::from_millis(100))
            .map_err(|e| crate::error::FemtoError::SandboxCreation(e))?
        {
            if let Event::Key(key_event) =
                event::read().map_err(|e| crate::error::FemtoError::SandboxCreation(e))?
            {
                // [v0.1.0] Windows에서 Press/Release/Repeat 이벤트가 모두 발생함.
                // Press만 처리해야 키 1회 입력 = 1회 동작이 보장된다.
                if key_event.kind == crossterm::event::KeyEventKind::Press {
                    app.handle_key(key_event);
                }
            }
        }

        // 비동기 작업 결과 확인 (API 검증 응답 등)
        app.tick();

        // 종료 조건 확인
        if !app.running {
            break;
        }
    }

    Ok(())
}
