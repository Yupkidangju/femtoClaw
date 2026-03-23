// femtoClaw — TUI 앱 상태 및 화면 렌더링
// [v0.1.0] Step 2: 앱 상태 머신, 키보드 입력 처리, 화면별 ratatui 렌더링,
// reqwest::blocking 기반 API 검증을 통합 관리한다.
//
// 화면 전환 흐름 (designs.md 3절):
// Boot → Password → (최초실행 ? Onboard : Dashboard) → Dashboard

use std::sync::mpsc;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use crate::config::{self, AppConfig, LlmPreset, LlmProviderConfig, TelegramConfig};
use crate::sandbox::SandboxPaths;

// === LLM 프리셋 정의 (spec.md 3.1절) ===

/// [v0.1.0] LLM 공급자 프리셋 데이터.
/// 2-Format, Multi-Preset 전략: API 클라이언트는 2개, 프리셋 이름은 8개.
struct Preset {
    name: &'static str,
    endpoint: &'static str,
    preset_type: LlmPreset,
}

/// 프리셋 목록 (좌우 방향키로 선택)
const PRESETS: &[Preset] = &[
    Preset {
        name: "OpenAI",
        endpoint: "https://api.openai.com/v1",
        preset_type: LlmPreset::OpenAi,
    },
    Preset {
        name: "Gemini",
        endpoint: "https://generativelanguage.googleapis.com/v1beta/openai",
        preset_type: LlmPreset::Gemini,
    },
    Preset {
        name: "Claude",
        endpoint: "https://api.anthropic.com/v1",
        preset_type: LlmPreset::Claude,
    },
    Preset {
        name: "xAI",
        endpoint: "https://api.x.ai/v1",
        preset_type: LlmPreset::XAi,
    },
    Preset {
        name: "OpenRouter",
        endpoint: "https://openrouter.ai/api/v1",
        preset_type: LlmPreset::OpenRouter,
    },
    Preset {
        name: "Ollama",
        endpoint: "http://localhost:11434",
        preset_type: LlmPreset::Ollama,
    },
    Preset {
        name: "LM Studio",
        endpoint: "http://localhost:1234/v1",
        preset_type: LlmPreset::LmStudio,
    },
    Preset {
        name: "Custom",
        endpoint: "",
        preset_type: LlmPreset::Custom,
    },
];

// === 입력 필드 글자 수 상한 ===
// [v0.1.0] 붙여넣기 등으로 대량 입력 시 UI 범위 초과 방지
const MAX_PASSWORD_LEN: usize = 128;
const MAX_API_KEY_LEN: usize = 256;
const MAX_MODEL_LEN: usize = 64;
const MAX_TOKEN_LEN: usize = 128;

// === 화면/상태 열거형 ===

/// 현재 활성 화면
#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Boot,
    Password,
    Onboard,
    Dashboard,
}

/// 온보딩 화면에서 현재 포커스된 입력 필드
#[derive(Debug, Clone, PartialEq)]
enum OnboardField {
    ApiKey,
    Model,
    TelegramToken,
}

/// API 검증 상태
#[derive(Debug, Clone, PartialEq)]
enum ValidationStatus {
    None,
    Testing,
    Ok,
    Failed(String),
}

/// 비동기 검증 결과 (mpsc 채널 전송용)
/// LLM 검증 성공 시 모델 목록도 함께 전달한다.
enum AsyncResult {
    LlmValidation(Result<(String, Vec<String>), String>),
    TelegramValidation(Result<String, String>),
}

// === 앱 구조체 ===

/// [v0.1.0] TUI 앱 전체 상태.
pub struct App {
    /// 앱 실행 중 여부 (false가 되면 메인 루프 종료)
    pub running: bool,

    // 경로 정보
    paths: SandboxPaths,

    // 화면 상태
    screen: Screen,
    boot_timer: u8,

    // 비밀번호 화면 상태
    password: String,
    password_confirm: String,
    pw_field_confirm: bool,
    pw_error: Option<String>,
    pw_attempts: u8,
    is_first_run: bool,

    // 온보딩 화면 상태
    preset_index: usize,
    endpoint: String,
    api_key: String,
    model: String,
    telegram_token: String,
    onboard_field: OnboardField,
    llm_status: ValidationStatus,
    tg_status: ValidationStatus,
    /// API에서 가져온 사용 가능한 모델 목록
    available_models: Vec<String>,
    /// 현재 선택된 모델 인덱스 (available_models 내)
    model_index: usize,

    // 비동기 검증 채널
    async_tx: mpsc::Sender<AsyncResult>,
    async_rx: mpsc::Receiver<AsyncResult>,

    // 설정 데이터
    app_config: AppConfig,

    // 대시보드 로그
    feed_lines: Vec<String>,
}

impl App {
    /// [v0.1.0] 새 앱 인스턴스를 생성한다.
    pub fn new(paths: SandboxPaths) -> Self {
        let (tx, rx) = mpsc::channel();
        let is_first = !config::config_exists(&paths.config_enc);
        let preset = &PRESETS[0];

        Self {
            running: true,
            paths,
            screen: Screen::Boot,
            boot_timer: 0,
            password: String::new(),
            password_confirm: String::new(),
            pw_field_confirm: false,
            pw_error: None,
            pw_attempts: 0,
            is_first_run: is_first,
            preset_index: 0,
            endpoint: preset.endpoint.to_string(),
            api_key: String::new(),
            model: String::new(),
            telegram_token: String::new(),
            onboard_field: OnboardField::ApiKey,
            llm_status: ValidationStatus::None,
            tg_status: ValidationStatus::None,
            available_models: Vec::new(),
            model_index: 0,
            async_tx: tx,
            async_rx: rx,
            app_config: AppConfig::default(),
            feed_lines: vec![format!("[{}] femtoClaw v0.1.0-beta 시작", timestamp())],
        }
    }

    // === 이벤트 처리 ===

    /// 키 이벤트를 현재 화면에 맞게 디스패치한다.
    pub fn handle_key(&mut self, key: KeyEvent) {
        // Ctrl+C는 어디서든 종료
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.running = false;
            return;
        }

        match self.screen {
            Screen::Boot => self.handle_boot_key(key),
            Screen::Password => self.handle_password_key(key),
            Screen::Onboard => self.handle_onboard_key(key),
            Screen::Dashboard => self.handle_dashboard_key(key),
        }
    }

    /// 비동기 작업 결과 확인 (매 tick마다 호출)
    pub fn tick(&mut self) {
        // 부트 타이머 처리 (빠른 부트: 5틱 = ~0.5초)
        if self.screen == Screen::Boot {
            self.boot_timer += 1;
            if self.boot_timer >= 5 {
                self.screen = Screen::Password;
            }
        }

        // 비동기 검증 결과 수신
        while let Ok(result) = self.async_rx.try_recv() {
            match result {
                AsyncResult::LlmValidation(Ok((msg, models))) => {
                    self.llm_status = ValidationStatus::Ok;
                    // 모델 목록 반영
                    if !models.is_empty() {
                        self.available_models = models;
                        self.model_index = 0;
                        self.model = self.available_models[0].clone();
                        self.feed_lines.push(format!(
                            "[{}] LLM 검증 성공: {} — {}개 모델 발견",
                            timestamp(),
                            msg,
                            self.available_models.len()
                        ));
                    } else {
                        self.feed_lines
                            .push(format!("[{}] LLM 검증 성공: {}", timestamp(), msg));
                    }
                }
                AsyncResult::LlmValidation(Err(e)) => {
                    self.llm_status = ValidationStatus::Failed(e.clone());
                    self.available_models.clear();
                    self.feed_lines
                        .push(format!("[{}] LLM 검증 실패: {}", timestamp(), e));
                }
                AsyncResult::TelegramValidation(Ok(msg)) => {
                    self.tg_status = ValidationStatus::Ok;
                    self.feed_lines
                        .push(format!("[{}] Telegram 검증 성공: {}", timestamp(), msg));
                }
                AsyncResult::TelegramValidation(Err(e)) => {
                    self.tg_status = ValidationStatus::Failed(e.clone());
                    self.feed_lines
                        .push(format!("[{}] Telegram 검증 실패: {}", timestamp(), e));
                }
            }
        }
    }

    // === 화면별 키 처리 ===

    fn handle_boot_key(&mut self, key: KeyEvent) {
        // ESC로 부트 시퀀스 스킵
        if key.code == KeyCode::Esc {
            self.screen = Screen::Password;
        }
    }

    fn handle_password_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.running = false,
            KeyCode::Tab => {
                if self.is_first_run {
                    self.pw_field_confirm = !self.pw_field_confirm;
                }
            }
            KeyCode::Enter => self.submit_password(),
            KeyCode::Backspace => {
                if self.pw_field_confirm {
                    self.password_confirm.pop();
                } else {
                    self.password.pop();
                }
            }
            KeyCode::Char(c) => {
                // 글자 수 상한 초과 시 무시 (붙여넣기 방어)
                if self.pw_field_confirm {
                    if self.password_confirm.len() < MAX_PASSWORD_LEN {
                        self.password_confirm.push(c);
                    }
                } else if self.password.len() < MAX_PASSWORD_LEN {
                    self.password.push(c);
                }
                self.pw_error = None;
            }
            _ => {}
        }
    }

    fn handle_onboard_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.running = false,
            KeyCode::Tab => {
                // 필드 순환: ApiKey → Model → TelegramToken → ApiKey
                self.onboard_field = match self.onboard_field {
                    OnboardField::ApiKey => OnboardField::Model,
                    OnboardField::Model => OnboardField::TelegramToken,
                    OnboardField::TelegramToken => OnboardField::ApiKey,
                };
            }
            KeyCode::Left => {
                // 프리셋 선택 (좌)
                if self.preset_index > 0 {
                    self.preset_index -= 1;
                    self.sync_preset();
                }
            }
            KeyCode::Right => {
                // 프리셋 선택 (우)
                if self.preset_index < PRESETS.len() - 1 {
                    self.preset_index += 1;
                    self.sync_preset();
                }
            }
            KeyCode::Up => {
                // Model 필드에서 ↑: 이전 모델 선택
                if self.onboard_field == OnboardField::Model && !self.available_models.is_empty() {
                    if self.model_index > 0 {
                        self.model_index -= 1;
                    } else {
                        self.model_index = self.available_models.len() - 1;
                    }
                    self.model = self.available_models[self.model_index].clone();
                }
            }
            KeyCode::Down => {
                // Model 필드에서 ↓: 다음 모델 선택
                if self.onboard_field == OnboardField::Model && !self.available_models.is_empty() {
                    if self.model_index < self.available_models.len() - 1 {
                        self.model_index += 1;
                    } else {
                        self.model_index = 0;
                    }
                    self.model = self.available_models[self.model_index].clone();
                }
            }
            KeyCode::Char('v') | KeyCode::Char('V') => {
                self.start_validation();
            }
            KeyCode::Enter => self.submit_onboard(),
            KeyCode::Backspace => self.onboard_backspace(),
            KeyCode::Char(c) => self.onboard_char(c),
            _ => {}
        }
    }

    fn handle_dashboard_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            KeyCode::Char('1') => {
                // [v0.2.0] 에이전트 상태 표시
                self.feed_lines
                    .push(format!("[{}] ━━ Agent Status ━━", timestamp()));
                self.feed_lines
                    .push(format!("  에이전트: {}", self.app_config.agent_name));
                if let Some(ref llm) = self.app_config.llm_provider {
                    self.feed_lines
                        .push(format!("  모델: {} ({:?})", llm.model, llm.preset));
                }
                self.feed_lines
                    .push(format!("  보안: Jailing=ON | ChaCha20=ON"));
            }
            KeyCode::Char('2') => {
                if let Some(ref llm) = self.app_config.llm_provider {
                    self.feed_lines.push(format!(
                        "[{}] Model API: {:?} / {} / {}",
                        timestamp(),
                        llm.preset,
                        llm.endpoint,
                        llm.model
                    ));
                } else {
                    self.feed_lines
                        .push(format!("[{}] LLM 미설정", timestamp()));
                }
            }
            KeyCode::Char('3') => {
                // [v0.2.0] 스킬 목록 표시 (TOML + Rhai 하이브리드)
                self.feed_lines
                    .push(format!("[{}] ━━ Skill List (TOML + Rhai) ━━", timestamp()));
                // skills/core/ 로드
                let core_dir = self.paths.skills_core.clone();
                let user_dir = self.paths.skills_user.clone();
                match crate::skills::load_skills_from_dir(&core_dir, true) {
                    Ok(core_skills) => {
                        for s in &core_skills {
                            let tag = match s.skill_type {
                                crate::skills::SkillType::Static => "TOML",
                                crate::skills::SkillType::Dynamic => "Rhai",
                            };
                            self.feed_lines
                                .push(format!("  [내장][{}] {} — {}", tag, s.name, s.description));
                        }
                    }
                    Err(e) => self.feed_lines.push(format!("  core 로드 실패: {}", e)),
                }
                match crate::skills::load_skills_from_dir(&user_dir, false) {
                    Ok(user_skills) => {
                        for s in &user_skills {
                            let tag = match s.skill_type {
                                crate::skills::SkillType::Static => "TOML",
                                crate::skills::SkillType::Dynamic => "Rhai",
                            };
                            self.feed_lines.push(format!(
                                "  [사용자][{}] {} — {}",
                                tag, s.name, s.description
                            ));
                        }
                    }
                    Err(e) => self.feed_lines.push(format!("  user 로드 실패: {}", e)),
                }
            }
            KeyCode::Char('4') => {
                // [v0.2.0] 타임머신 — DB에서 최근 10건 조회
                self.feed_lines
                    .push(format!("[{}] ━━ Time Machine (최근 10건) ━━", timestamp()));
                self.feed_lines.push(format!(
                    "  {:>4} | {:>6} | {:>6} | {:<30} | {}",
                    "#", "유형", "상태", "요약", "시각"
                ));
                self.feed_lines.push(format!("  {}", "─".repeat(70)));
                let db_path = self.paths.db_dir.join("femto_state.db");
                match crate::db::FemtoDb::open(&db_path) {
                    Ok(db) => match db.actions_paged(0, 10) {
                        Ok(records) => {
                            if records.is_empty() {
                                self.feed_lines.push("  (기록 없음)".to_string());
                            }
                            for r in &records {
                                let status = if r.undone { "↩ Undo" } else { "✅ 완료" };
                                let summary_trunc = if r.summary.len() > 28 {
                                    format!("{}...", &r.summary[..28])
                                } else {
                                    r.summary.clone()
                                };
                                self.feed_lines.push(format!(
                                    "  {:>4} | {:>6} | {:>6} | {:<30} | {}",
                                    r.id,
                                    r.action_type.display_name(),
                                    status,
                                    summary_trunc,
                                    r.timestamp
                                ));
                            }
                            match db.action_count() {
                                Ok(count) => {
                                    self.feed_lines.push(format!("  ── 전체 {} 건 ──", count));
                                }
                                Err(_) => {}
                            }
                        }
                        Err(e) => {
                            self.feed_lines.push(format!("  DB 조회 실패: {}", e));
                        }
                    },
                    Err(e) => {
                        self.feed_lines.push(format!("  DB 열기 실패: {}", e));
                    }
                }
            }
            KeyCode::Char('5') => {
                // [v0.3.0] 에이전트 전환
                self.feed_lines
                    .push(format!("[{}] ━━ Agent Switch ━━", timestamp()));
                if self.app_config.agents.is_empty() {
                    self.feed_lines.push("  (등록된 에이전트 없음)".to_string());
                } else {
                    for a in &self.app_config.agents {
                        let marker = if a.id == self.app_config.active_agent_id {
                            "▶"
                        } else {
                            " "
                        };
                        let status = if a.active { "활성" } else { "비활성" };
                        self.feed_lines.push(format!(
                            "  {} Agent #{}: {} ({})",
                            marker, a.id, a.name, status
                        ));
                    }
                    // 다음 에이전트로 순환 전환
                    let current = self.app_config.active_agent_id;
                    let active_ids: Vec<u8> = self
                        .app_config
                        .agents
                        .iter()
                        .filter(|a| a.active)
                        .map(|a| a.id)
                        .collect();
                    if active_ids.len() > 1 {
                        let pos = active_ids.iter().position(|&id| id == current).unwrap_or(0);
                        let next = active_ids[(pos + 1) % active_ids.len()];
                        self.app_config.active_agent_id = next;
                        // 이름을 먼저 복사하여 borrow 해제
                        let agent_name = self
                            .app_config
                            .active_agent()
                            .map(|a| a.name.clone())
                            .unwrap_or_default();
                        if !agent_name.is_empty() {
                            self.app_config.agent_name = agent_name.clone();
                            self.feed_lines
                                .push(format!("  → 에이전트 #{}({})로 전환!", next, agent_name));
                        }
                    } else {
                        self.feed_lines
                            .push("  (전환 가능한 다른 에이전트 없음)".to_string());
                    }
                }
            }
            KeyCode::Char('a') => {
                // [v0.3.0] 에이전트 추가
                let names = ["Alpha", "Beta", "Gamma"];
                let next_name = names.get(self.app_config.agents.len()).unwrap_or(&"Agent");
                match self.app_config.add_agent(next_name) {
                    Ok(id) => {
                        self.feed_lines.push(format!(
                            "[{}] ✅ 에이전트 #{} ({}) 추가 완료",
                            timestamp(),
                            id,
                            next_name
                        ));
                    }
                    Err(e) => {
                        self.feed_lines.push(format!("[{}] ❌ {}", timestamp(), e));
                    }
                }
            }
            _ => {}
        }
    }

    // === 비밀번호 로직 ===

    fn submit_password(&mut self) {
        if self.is_first_run {
            // 최초 실행: 두 필드 일치 검증
            if self.password.is_empty() {
                self.pw_error = Some("비밀번호를 입력하세요".to_string());
                return;
            }
            if self.password.len() < 4 {
                self.pw_error = Some("최소 4자 이상 입력하세요".to_string());
                return;
            }
            if self.password != self.password_confirm {
                self.pw_error = Some("비밀번호가 일치하지 않습니다".to_string());
                return;
            }
            // 기본 설정으로 config.enc 생성
            let _ = config::save_config(
                &self.app_config,
                self.password.as_bytes(),
                &self.paths.config_enc,
            );
            self.feed_lines
                .push(format!("[{}] 마스터 키 생성 완료", timestamp()));
            self.screen = Screen::Onboard;
        } else {
            // 재실행: 기존 config.enc 복호화 시도
            match config::load_config(self.password.as_bytes(), &self.paths.config_enc) {
                Ok(cfg) => {
                    self.app_config = cfg;
                    self.feed_lines
                        .push(format!("[{}] 설정 복호화 성공", timestamp()));
                    self.screen = Screen::Dashboard;
                }
                Err(_) => {
                    self.pw_attempts += 1;
                    if self.pw_attempts >= 3 {
                        self.pw_error = Some("3회 실패. [R]을 눌러 설정을 리셋하세요".to_string());
                    } else {
                        self.pw_error = Some(format!("비밀번호 오류 ({}/3)", self.pw_attempts));
                    }
                    self.password.clear();
                }
            }
        }
    }

    // === 온보딩 로직 ===

    fn sync_preset(&mut self) {
        let preset = &PRESETS[self.preset_index];
        self.endpoint = preset.endpoint.to_string();
        self.llm_status = ValidationStatus::None;
    }

    fn onboard_char(&mut self, c: char) {
        // 글자 수 상한 초과 시 무시 (붙여넣기 방어)
        match self.onboard_field {
            OnboardField::ApiKey => {
                if self.api_key.len() < MAX_API_KEY_LEN {
                    self.api_key.push(c);
                }
            }
            OnboardField::Model => {
                if self.model.len() < MAX_MODEL_LEN {
                    self.model.push(c);
                }
            }
            OnboardField::TelegramToken => {
                if self.telegram_token.len() < MAX_TOKEN_LEN {
                    self.telegram_token.push(c);
                }
            }
        }
    }

    fn onboard_backspace(&mut self) {
        match self.onboard_field {
            OnboardField::ApiKey => {
                self.api_key.pop();
            }
            OnboardField::Model => {
                self.model.pop();
            }
            OnboardField::TelegramToken => {
                self.telegram_token.pop();
            }
        }
    }

    /// [v0.1.0] API 키 검증을 비동기로 시작한다.
    /// reqwest::blocking을 별도 스레드에서 실행하여 TUI를 블로킹하지 않는다.
    fn start_validation(&mut self) {
        // LLM 검증 (테스트 중에도 재시도 허용 — 기존 스레드 결과는 무시됨)
        if !self.api_key.is_empty() {
            self.llm_status = ValidationStatus::Testing;
            let endpoint = self.endpoint.clone();
            let api_key = self.api_key.clone();
            let preset = PRESETS[self.preset_index].preset_type.clone();
            let tx = self.async_tx.clone();

            std::thread::spawn(move || {
                let result = validate_llm_api(&endpoint, &api_key, &preset);
                let _ = tx.send(AsyncResult::LlmValidation(result));
            });
        }

        // 텔레그램 검증 (테스트 중에도 재시도 허용)
        if !self.telegram_token.is_empty() {
            self.tg_status = ValidationStatus::Testing;
            let token = self.telegram_token.clone();
            let tx = self.async_tx.clone();

            std::thread::spawn(move || {
                let result = validate_telegram(&token);
                let _ = tx.send(AsyncResult::TelegramValidation(result));
            });
        }
    }

    /// 온보딩 완료: 검증된 설정을 config.enc에 저장한다.
    /// spec.md 5절: 유효성 검증 없이 Config 저장 금지.
    fn submit_onboard(&mut self) {
        // LLM은 반드시 검증 완료되어야 진행 가능
        if self.llm_status != ValidationStatus::Ok {
            return;
        }

        let preset = &PRESETS[self.preset_index];
        self.app_config.llm_provider = Some(LlmProviderConfig {
            preset: preset.preset_type.clone(),
            endpoint: self.endpoint.clone(),
            api_key: self.api_key.clone(),
            model: self.model.clone(),
            verified: true,
        });

        // 텔레그램은 선택사항 (검증 완료 시에만 저장)
        if self.tg_status == ValidationStatus::Ok {
            self.app_config.telegram = Some(TelegramConfig {
                bot_token: self.telegram_token.clone(),
                chat_id: None,
                verified: true,
            });
        }

        // config.enc 저장
        let _ = config::save_config(
            &self.app_config,
            self.password.as_bytes(),
            &self.paths.config_enc,
        );

        self.feed_lines
            .push(format!("[{}] 설정 저장 완료 → 대시보드", timestamp()));
        self.screen = Screen::Dashboard;
    }

    // === 화면 렌더링 ===

    /// 현재 화면에 맞는 렌더링 함수를 호출한다.
    pub fn render(&self, frame: &mut Frame) {
        // 전체 배경 색상 설정
        let bg = Block::default().style(ratatui::style::Style::default().bg(theme::BACKGROUND));
        frame.render_widget(bg, frame.area());

        match self.screen {
            Screen::Boot => self.render_boot(frame),
            Screen::Password => self.render_password(frame),
            Screen::Onboard => self.render_onboard(frame),
            Screen::Dashboard => self.render_dashboard(frame),
        }
    }

    // --- 부트 화면 ---
    fn render_boot(&self, frame: &mut Frame) {
        let area = frame.area();
        let lines = vec![
            Line::from(Span::styled(
                "BIOS DATE 03/23/26 00:00:00 VER 0.1",
                theme::title(),
            )),
            Line::from(""),
            Line::from(Span::styled("femtoClaw (C) 2026", theme::title())),
            Line::from(""),
            Line::from(Span::styled(
                "SYSTEM: RUST COMPATIBLE @ ZERO-COST",
                theme::title(),
            )),
            Line::from(Span::styled(
                format!("BASE MEMORY....{:>42}OK", ""),
                theme::text(),
            )),
            Line::from(""),
        ];

        // 부트 타이머에 따라 라인을 점진적으로 표시
        let visible = (self.boot_timer as usize).min(lines.len());
        let visible_lines: Vec<Line> = lines.into_iter().take(visible).collect();

        let boot = Paragraph::new(visible_lines).style(theme::text());
        frame.render_widget(boot, area);

        // 하단 ESC 힌트
        let hint = Paragraph::new("PRESS [ESC] TO SKIP").style(theme::muted());
        let hint_area = Rect::new(0, area.height.saturating_sub(1), area.width, 1);
        frame.render_widget(hint, hint_area);
    }

    // --- 비밀번호 화면 ---
    fn render_password(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(3), // 타이틀 바
            Constraint::Min(0),    // 컨텐츠
            Constraint::Length(1), // 푸터
        ])
        .split(area);

        // 타이틀 바
        let title_text = if self.is_first_run {
            "femtoClaw v0.1 BETA — Master Password Setup"
        } else {
            "femtoClaw v0.1 BETA — Enter Master Password"
        };
        let title = Paragraph::new(title_text)
            .style(theme::title())
            .block(Block::bordered().border_style(theme::border()));
        frame.render_widget(title, chunks[0]);

        // 컨텐츠 영역
        let content = chunks[1];
        let inner = center_rect(content, 60, if self.is_first_run { 14 } else { 10 });

        let mut lines: Vec<Line> = vec![
            Line::from(""),
            Line::from(Span::styled("  [!] Welcome to femtoClaw.", theme::title())),
            Line::from(Span::styled(
                "      Workspace jailed at: ~/.femtoclaw/workspace/",
                theme::text(),
            )),
            Line::from(""),
        ];

        // 비밀번호 필드 (박스 폭 40글자 제한)
        let pw_display = truncate_for_display(&"*".repeat(self.password.len()), 40);
        let pw_indicator = if !self.pw_field_confirm { "▶ " } else { "  " };
        lines.push(Line::from(vec![
            Span::styled(format!("  {}Password : [", pw_indicator), theme::text()),
            Span::styled(pw_display, theme::input()),
            Span::styled("]", theme::text()),
        ]));

        if self.is_first_run {
            let cf_display = truncate_for_display(&"*".repeat(self.password_confirm.len()), 40);
            let cf_indicator = if self.pw_field_confirm { "▶ " } else { "  " };
            lines.push(Line::from(vec![
                Span::styled(format!("  {}Confirm  : [", cf_indicator), theme::text()),
                Span::styled(cf_display, theme::input()),
                Span::styled("]", theme::text()),
            ]));
        }

        // 에러 메시지
        if let Some(ref err) = self.pw_error {
            lines.push(Line::from(""));
            lines.push(Line::from(Span::styled(
                format!("  ✗ {}", err),
                theme::error(),
            )));
        }

        let pw_block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme::active_border());
        let pw_widget = Paragraph::new(lines).block(pw_block).style(theme::text());
        frame.render_widget(pw_widget, inner);

        // 푸터
        let footer_text = if self.is_first_run {
            "[Enter] Confirm & Generate Key    [Tab] Switch Field    [Esc] Quit"
        } else {
            "[Enter] Unlock    [Esc] Quit"
        };
        let footer = Paragraph::new(footer_text).style(theme::muted());
        frame.render_widget(footer, chunks[2]);
    }

    // --- 온보딩 화면 ---
    fn render_onboard(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(3), // 타이틀
            Constraint::Min(0),    // 컨텐츠
            Constraint::Length(1), // 푸터
        ])
        .split(area);

        // 타이틀
        let title = Paragraph::new("femtoClaw v0.1 BETA — Onboarding: API & Token Setup")
            .style(theme::title())
            .block(Block::bordered().border_style(theme::border()));
        frame.render_widget(title, chunks[0]);

        // 컨텐츠 — LLM + Telegram 섹션
        let content = chunks[1];
        let sections = Layout::vertical([
            Constraint::Length(10), // LLM 섹션
            Constraint::Length(7),  // Telegram 섹션
            Constraint::Min(0),     // 여백
        ])
        .split(content);

        self.render_llm_section(frame, sections[0]);
        self.render_telegram_section(frame, sections[1]);

        // 푸터
        let footer = Paragraph::new(
            "[←/→] Preset    [Tab] Field    [V] Verify    [Enter] Save & Continue    [Esc] Quit",
        )
        .style(theme::muted());
        frame.render_widget(footer, chunks[2]);
    }

    fn render_llm_section(&self, frame: &mut Frame, area: Rect) {
        let _preset = &PRESETS[self.preset_index];

        // 프리셋 이름 목록 (현재 선택 강조)
        let preset_names: Vec<Span> = PRESETS
            .iter()
            .enumerate()
            .map(|(i, p)| {
                if i == self.preset_index {
                    Span::styled(format!(" [{}] ", p.name), theme::selected())
                } else {
                    Span::styled(format!("  {}  ", p.name), theme::muted())
                }
            })
            .collect();

        let api_indicator = if self.onboard_field == OnboardField::ApiKey {
            "▶ "
        } else {
            "  "
        };
        let model_indicator = if self.onboard_field == OnboardField::Model {
            "▶ "
        } else {
            "  "
        };

        let status_line = match &self.llm_status {
            ValidationStatus::None => {
                Line::from(Span::styled("  Status: [—] 검증 대기", theme::muted()))
            }
            ValidationStatus::Testing => Line::from(Span::styled(
                "  Status: [⚙ TESTING...] 검증 중 (최대 5초)",
                theme::testing(),
            )),
            ValidationStatus::Ok => Line::from(Span::styled(
                "  Status: [✓ OK] 200 OK — Ready to save",
                theme::success(),
            )),
            ValidationStatus::Failed(e) => Line::from(Span::styled(
                format!("  Status: [✗ FAIL] {} — [V]로 재시도", e),
                theme::error(),
            )),
        };

        // 모델 필드: 목록이 있으면 선택기, 없으면 수동 입력
        let model_display = if !self.available_models.is_empty() {
            format!(
                "{} ({}/{})",
                truncate_for_display(&self.model, 40),
                self.model_index + 1,
                self.available_models.len()
            )
        } else {
            truncate_for_display(&self.model, 50)
        };
        let model_hint = if !self.available_models.is_empty() {
            " ↑↓"
        } else {
            ""
        };

        let lines = vec![
            Line::from(Span::styled(
                "  [ LLM Provider Configuration ]",
                theme::title(),
            )),
            Line::from(preset_names),
            Line::from(Span::styled(
                format!("  Endpoint: {}", &self.endpoint),
                theme::text(),
            )),
            Line::from(vec![
                Span::styled(format!("  {}API Key : [", api_indicator), theme::text()),
                Span::styled(truncate_for_display(&self.api_key, 50), theme::input()),
                Span::styled("]", theme::text()),
            ]),
            Line::from(vec![
                Span::styled(format!("  {}Model   : [", model_indicator), theme::text()),
                Span::styled(model_display, theme::input()),
                Span::styled(format!("]{}", model_hint), theme::text()),
            ]),
            status_line,
        ];

        let block = Block::bordered()
            .title(Span::styled("─ LLM ─", theme::title()))
            .border_style(theme::border());
        let widget = Paragraph::new(lines).block(block);
        frame.render_widget(widget, area);
    }

    fn render_telegram_section(&self, frame: &mut Frame, area: Rect) {
        let tg_indicator = if self.onboard_field == OnboardField::TelegramToken {
            "▶ "
        } else {
            "  "
        };

        let status_line = match &self.tg_status {
            ValidationStatus::None => Line::from(Span::styled(
                "  Status: [—] 검증 대기 (선택사항)",
                theme::muted(),
            )),
            ValidationStatus::Testing => Line::from(Span::styled(
                "  Status: [⚙ TESTING...] 검증 중 (최대 5초)",
                theme::testing(),
            )),
            ValidationStatus::Ok => Line::from(Span::styled(
                "  Status: [✓ OK] Telegram Bot 확인됨",
                theme::success(),
            )),
            ValidationStatus::Failed(e) => Line::from(Span::styled(
                format!("  Status: [✗ FAIL] {} — [V]로 재시도", e),
                theme::error(),
            )),
        };

        let lines = vec![
            Line::from(Span::styled(
                "  [ Telegram Bot Configuration ]",
                theme::title(),
            )),
            Line::from(vec![
                Span::styled(format!("  {}Bot Token: [", tg_indicator), theme::text()),
                Span::styled(
                    truncate_for_display(&self.telegram_token, 50),
                    theme::input(),
                ),
                Span::styled("]", theme::text()),
            ]),
            status_line,
        ];

        let block = Block::bordered()
            .title(Span::styled("─ TELEGRAM ─", theme::title()))
            .border_style(theme::border());
        let widget = Paragraph::new(lines).block(block);
        frame.render_widget(widget, area);
    }

    // --- 대시보드 화면 ---
    fn render_dashboard(&self, frame: &mut Frame) {
        let area = frame.area();

        // 전체: 상단 헤더 + 메인(좌/우 분할) + 하단 상태바
        let outer = Layout::vertical([
            Constraint::Length(1), // 헤더
            Constraint::Min(0),    // 메인
            Constraint::Length(1), // 상태바
        ])
        .split(area);

        // 헤더
        let provider_name = self
            .app_config
            .llm_provider
            .as_ref()
            .map(|p| format!("{:?}", p.preset))
            .unwrap_or_else(|| "—".into());
        let header = Paragraph::new(Line::from(vec![
            Span::styled(" femtoClaw Dashboard ", theme::status_bar()),
            Span::styled(format!(" │ Model: {} ", provider_name), theme::title()),
            Span::styled(" │ Status: [SECURE] │ Jailed: [ON] ", theme::text()),
        ]));
        frame.render_widget(header, outer[0]);

        // 메인 — 좌(시스템)/우(터미널) 분할
        let main = Layout::horizontal([Constraint::Length(28), Constraint::Min(0)]).split(outer[1]);

        // 좌측: 시스템 정보 + 메뉴
        let sys_lines = vec![
            Line::from(Span::styled(" AGENT", theme::title())),
            Line::from(Span::styled(
                format!("  Name: {}", self.app_config.agent_name),
                theme::text(),
            )),
            Line::from(Span::styled(
                format!("  Model: {}", provider_name),
                theme::text(),
            )),
            Line::from(""),
            Line::from(Span::styled(" MENU", theme::title())),
            Line::from(Span::styled("  [1] Agent Status", theme::text())),
            Line::from(Span::styled("  [2] Model APIs", theme::text())),
            Line::from(Span::styled("  [3] Skills (TOML+Rhai)", theme::text())),
            Line::from(Span::styled("  [4] Time Machine", theme::text())),
            Line::from(Span::styled("  [5] Agent Switch", theme::text())),
            Line::from(Span::styled("  [A] Add Agent", theme::text())),
        ];
        let sys_block = Block::bordered()
            .title(Span::styled("─ SYSTEM ─", theme::title()))
            .border_style(theme::border());
        let sys_widget = Paragraph::new(sys_lines).block(sys_block);
        frame.render_widget(sys_widget, main[0]);

        // 우측: 터미널 피드
        let visible_lines: usize = main[1].height.saturating_sub(2) as usize;
        let start = self.feed_lines.len().saturating_sub(visible_lines);
        let feed: Vec<Line> = self.feed_lines[start..]
            .iter()
            .map(|l| {
                if l.contains("BLOCKED") {
                    Line::from(Span::styled(l.as_str(), theme::error()))
                } else {
                    Line::from(Span::styled(l.as_str(), theme::text()))
                }
            })
            .collect();

        let feed_block = Block::bordered()
            .title(Span::styled("─ TERMINAL ─", theme::title()))
            .border_style(theme::border());
        let feed_widget = Paragraph::new(feed)
            .block(feed_block)
            .wrap(Wrap { trim: false });
        frame.render_widget(feed_widget, main[1]);

        // 상태바
        let status =
            Paragraph::new(" [Q] Quit  [1-5] Menu  [A] Add Agent  [U] Undo").style(theme::muted());
        frame.render_widget(status, outer[2]);
    }
}

// === API 검증 함수 ===

/// [v0.1.0] LLM API 키를 검증한다.
/// OpenAI 호환: GET /models 엔드포인트로 키 유효성 확인.
/// Ollama: GET /api/tags (인증 불필요, 서버 가동 확인).
fn validate_llm_api(
    endpoint: &str,
    api_key: &str,
    preset: &LlmPreset,
) -> Result<(String, Vec<String>), String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    let response = match preset {
        LlmPreset::Ollama | LlmPreset::LmStudio => {
            client.get(format!("{}/api/tags", endpoint)).send()
        }
        _ => client
            .get(format!("{}/models", endpoint))
            .header("Authorization", format!("Bearer {}", api_key))
            .send(),
    };

    match response {
        Ok(resp) if resp.status().is_success() => {
            let status_msg = format!("{} (200 OK)", resp.status());
            // 응답 본문에서 모델 목록 파싱
            let models = if let Ok(body) = resp.json::<serde_json::Value>() {
                parse_model_list(&body, preset)
            } else {
                Vec::new()
            };
            Ok((status_msg, models))
        }
        Ok(resp) => Err(format!("HTTP {}", resp.status())),
        Err(e) => {
            if e.is_timeout() {
                Err("타임아웃 (5초)".to_string())
            } else if e.is_connect() {
                Err("연결 실패 — 서버가 실행 중인지 확인하세요".to_string())
            } else {
                Err(format!("{}", e))
            }
        }
    }
}

/// [v0.1.0] API 응답 JSON에서 모델 목록을 추출한다.
/// OpenAI 호환: {"data": [{"id": "gpt-4"}, ...]}
/// Ollama: {"models": [{"name": "llama3"}, ...]}
fn parse_model_list(body: &serde_json::Value, preset: &LlmPreset) -> Vec<String> {
    let mut models = Vec::new();

    match preset {
        LlmPreset::Ollama | LlmPreset::LmStudio => {
            // Ollama 형식: models[].name
            if let Some(arr) = body.get("models").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(name) = item.get("name").and_then(|v| v.as_str()) {
                        models.push(name.to_string());
                    }
                }
            }
        }
        _ => {
            // OpenAI 호환 형식: data[].id
            if let Some(arr) = body.get("data").and_then(|v| v.as_array()) {
                for item in arr {
                    if let Some(id) = item.get("id").and_then(|v| v.as_str()) {
                        models.push(id.to_string());
                    }
                }
            }
        }
    }

    // 알파벳 정렬
    models.sort();
    models
}

/// [v0.1.0] 텔레그램 봇 토큰을 검증한다.
/// getMe API를 호출하여 봇 정보를 확인한다.
fn validate_telegram(token: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::builder()
        .connect_timeout(std::time::Duration::from_secs(3))
        .timeout(std::time::Duration::from_secs(5))
        .build()
        .map_err(|e| format!("HTTP 클라이언트 생성 실패: {}", e))?;

    let url = format!("https://api.telegram.org/bot{}/getMe", token);
    match client.get(&url).send() {
        Ok(resp) if resp.status().is_success() => Ok("봇 확인됨".to_string()),
        Ok(resp) => Err(format!(
            "HTTP {} — 토큰이 올바른지 확인하세요",
            resp.status()
        )),
        Err(e) => Err(format!("{}", e)),
    }
}

// === 유틸리티 ===

/// 간단한 타임스탬프 (HH:MM:SS)
fn timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let h = (now / 3600) % 24;
    let m = (now / 60) % 60;
    let s = now % 60;
    format!("{:02}:{:02}:{:02}", h, m, s)
}

/// 영역 중앙에 지정 크기의 사각형을 계산한다.
fn center_rect(area: Rect, width: u16, height: u16) -> Rect {
    let x = area.x + area.width.saturating_sub(width) / 2;
    let y = area.y + area.height.saturating_sub(height) / 2;
    Rect::new(x, y, width.min(area.width), height.min(area.height))
}

/// [v0.1.0] 입력 텍스트가 표시 영역을 초과할 경우 끝부분만 보이도록 자른다.
/// max_width: 보여줄 최대 문자 수. 초과 시 앞에 "…"를 붙여 tail 표시.
fn truncate_for_display(text: &str, max_width: usize) -> String {
    if text.len() <= max_width {
        text.to_string()
    } else {
        // 끝에서 (max_width - 1)글자만 보여주고 앞에 … 표시
        let start = text.len() - (max_width.saturating_sub(1));
        format!("…{}", &text[start..])
    }
}
