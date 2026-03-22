// femtoClaw — Amber Monochrome 디자인 토큰
// [v0.1.0] Step 2: Design 2 (Midnight Commander / htop 스타일) 색상 및 스타일 정의.
// design/2/retro_terminal_amber_monochrome_prd.html의 디자인 토큰을 Rust 상수로 매핑.

use ratatui::style::{Color, Modifier, Style};

// === 색상 팔레트 (PRD 디자인 토큰) ===
/// 주요 텍스트, 보더, 활성 인디케이터 (Amber)
pub const PRIMARY: Color = Color::Rgb(0xFF, 0xB0, 0x00);
/// 최심부 배경 (매우 어두운 갈색/검정)
pub const BACKGROUND: Color = Color::Rgb(0x0A, 0x07, 0x00);
/// 모달/패널 배경
pub const SURFACE: Color = Color::Rgb(0x14, 0x0E, 0x00);
/// 본문 텍스트, 터미널 출력
pub const TEXT: Color = Color::Rgb(0xFF, 0xC2, 0x33);
/// 박스 드로잉 보더, 비활성 텍스트
pub const MUTED: Color = Color::Rgb(0x66, 0x46, 0x00);
/// 고강도 텍스트, 하이라이트
pub const ACCENT: Color = Color::Rgb(0xFF, 0xD6, 0x66);

// === 시맨틱 색상 ===
/// 성공 상태 표시
pub const SUCCESS: Color = Color::Rgb(0x55, 0xFF, 0x55);
/// 에러 상태 표시
pub const ERROR: Color = Color::Rgb(0xFF, 0x44, 0x44);
/// 테스트 중 상태 표시
pub const TESTING: Color = Color::Rgb(0x55, 0xFF, 0xFF);

// === 스타일 헬퍼 함수 ===

/// 제목/헤더 스타일 (Primary + Bold)
pub fn title() -> Style {
    Style::default().fg(PRIMARY).add_modifier(Modifier::BOLD)
}

/// 본문 텍스트 스타일
pub fn text() -> Style {
    Style::default().fg(TEXT)
}

/// 비활성/보더 스타일
pub fn muted() -> Style {
    Style::default().fg(MUTED)
}

/// 입력 필드 스타일 (Accent)
pub fn input() -> Style {
    Style::default().fg(ACCENT)
}

/// 성공 메시지 스타일
pub fn success() -> Style {
    Style::default().fg(SUCCESS)
}

/// 에러 메시지 스타일
pub fn error() -> Style {
    Style::default().fg(ERROR)
}

/// 테스트 중 스타일
pub fn testing() -> Style {
    Style::default().fg(TESTING)
}

/// 상태바 스타일 (반전: Amber 배경 + 검정 텍스트)
pub fn status_bar() -> Style {
    Style::default().bg(PRIMARY).fg(BACKGROUND)
}

/// 선택된 항목 스타일 (반전)
pub fn selected() -> Style {
    Style::default()
        .bg(PRIMARY)
        .fg(BACKGROUND)
        .add_modifier(Modifier::BOLD)
}

/// 보더 스타일 (Muted 단선 테두리)
pub fn border() -> Style {
    Style::default().fg(MUTED)
}

/// 활성 보더 스타일 (Primary)
pub fn active_border() -> Style {
    Style::default().fg(PRIMARY)
}
