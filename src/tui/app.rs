// femtoClaw — TUI 앱 상태 및 화면 렌더링
// [v0.1.0] Step 2: 앱 상태 머신, 키보드 입력 처리, 화면별 ratatui 렌더링,
// reqwest::blocking 기반 API 검증을 통합 관리한다.
// [v0.6.0] Dashboard 채팅 패널 추가 — handle_message() 연동
//
// 화면 전환 흐름 (designs.md 3절):
// Boot → Dashboard (온보딩 미완료 시 Onboard)

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
// [v1.1.0] MAX_PASSWORD_LEN 삭제됨 — 비밀번호 기능 제거
const MAX_API_KEY_LEN: usize = 256;
const MAX_MODEL_LEN: usize = 64;
const MAX_TOKEN_LEN: usize = 128;

// === 화면/상태 열거형 ===

/// 현재 활성 화면
#[derive(Debug, Clone, PartialEq)]
enum Screen {
    Boot,
    Onboard,
    Dashboard,
    /// [v1.1.0] 설정 수정 화면
    Settings,
    /// [v1.1.0] 에이전트 편집 화면
    EditAgent,
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

    /// [v1.1.0] 초기 온보딩 완료 여부
    is_first_run: bool,
    /// [v1.1.0] Settings 화면에서 선택된 항목 인덱스
    settings_index: usize,
    /// [v1.1.0] EditAgent 화면에서 선택된 에이전트 인덱스
    edit_agent_index: usize,

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

    // [v0.7.0] 채팅 상태
    /// 채팅 입력 모드 활성화 여부
    chat_mode: bool,
    /// 채팅 입력 버퍼
    chat_input: String,
    /// 채팅 기록 (표시용)
    chat_history: Vec<(String, String)>, // (role, content)
    /// [v0.7.0] 비동기 채팅 워커 (background thread)
    chat_worker: Option<crate::core::chat_loop::ChatWorker>,
    /// [v0.7.0] LLM 응답 대기 중 여부
    chat_waiting: bool,
    /// [v1.0.0] 멀티 에이전트 경로 관리자
    agent_manager: Option<crate::core::agent_manager::AgentManager>,
}

impl App {
    /// [v0.1.0] 새 앱 인스턴스를 생성한다.
    pub fn new(paths: SandboxPaths) -> Self {
        let (tx, rx) = mpsc::channel();
        let is_first = !config::config_exists(&paths.config_enc);
        let preset = &PRESETS[0];

        // [v1.1.0] 기존 config.enc가 있으면 고정 키로 로드 (설정 영속화)
        let app_config = if !is_first {
            config::load_config(b"femtoclaw-default-key", &paths.config_enc)
                .unwrap_or_else(|_| AppConfig::default())
        } else {
            AppConfig::default()
        };

        // [v1.0.0] 멀티 에이전트 경로 관리자 — paths move 전에 먼저 생성
        let agent_manager = {
            let ids: Vec<u8> = app_config.agents.iter().map(|a| a.id).collect();
            crate::core::agent_manager::AgentManager::new(paths.root.clone(), &ids).ok()
        };

        Self {
            running: true,
            paths,
            screen: Screen::Boot,
            boot_timer: 0,
            is_first_run: is_first,
            settings_index: 0,
            edit_agent_index: 0,
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
            app_config,
            feed_lines: vec![format!("[{}] {}", timestamp(), msg!("boot.init_msg"))],
            chat_mode: false,
            chat_input: String::new(),
            chat_history: Vec::new(),
            chat_worker: None,
            chat_waiting: false,
            agent_manager,
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
            Screen::Settings => self.handle_settings_key(key),
            Screen::EditAgent => self.handle_edit_agent_key(key),
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
                // [v1.1.0] 비밀번호 없이 직행 — 온보딩 미완료 시 Onboard
                if self.app_config.llm_provider.is_none() {
                    self.screen = Screen::Onboard;
                } else {
                    self.screen = Screen::Dashboard;
                }
            }
        }

        // 비동기 검증 결과 수신
        while let Ok(result) = self.async_rx.try_recv() {
            match result {
                AsyncResult::LlmValidation(Ok((m, models))) => {
                    self.llm_status = ValidationStatus::Ok;
                    // 모델 목록 반영
                    if !models.is_empty() {
                        self.available_models = models;
                        self.model_index = 0;
                        self.model = self.available_models[0].clone();
                        self.feed_lines.push(format!(
                            "[{}] {}",
                            timestamp(),
                            msg!("feed.llm_verify_ok", m, self.available_models.len())
                        ));
                    } else {
                        self.feed_lines.push(format!(
                            "[{}] {}",
                            timestamp(),
                            msg!("feed.llm_verify_ok_simple", m)
                        ));
                    }
                }
                AsyncResult::LlmValidation(Err(e)) => {
                    self.llm_status = ValidationStatus::Failed(e.clone());
                    self.available_models.clear();
                    self.feed_lines.push(format!(
                        "[{}] {}",
                        timestamp(),
                        msg!("feed.llm_verify_fail", e)
                    ));
                }
                AsyncResult::TelegramValidation(Ok(m)) => {
                    self.tg_status = ValidationStatus::Ok;
                    self.feed_lines.push(format!(
                        "[{}] {}",
                        timestamp(),
                        msg!("feed.tg_verify_ok", m)
                    ));
                }
                AsyncResult::TelegramValidation(Err(e)) => {
                    self.tg_status = ValidationStatus::Failed(e.clone());
                    self.feed_lines.push(format!(
                        "[{}] {}",
                        timestamp(),
                        msg!("feed.tg_verify_fail", e)
                    ));
                }
            }
        }

        // [v0.7.0] 채팅 워커 비동기 응답 수신
        if let Some(ref worker) = self.chat_worker {
            while let Some(event) = worker.try_recv() {
                match event {
                    crate::core::chat_loop::ChatEvent::Thinking => {
                        self.chat_waiting = true;
                        self.chat_history
                            .push(("system".into(), "🤔 Thinking...".into()));
                    }
                    crate::core::chat_loop::ChatEvent::Reply(reply) => {
                        self.chat_waiting = false;
                        // "Thinking..." 제거
                        if let Some(last) = self.chat_history.last() {
                            if last.1 == "🤔 Thinking..." {
                                self.chat_history.pop();
                            }
                        }
                        self.chat_history.push(("assistant".into(), reply.clone()));
                        self.feed_lines.push(format!(
                            "[{}] Agent: {}",
                            timestamp(),
                            reply.chars().take(100).collect::<String>()
                        ));
                    }
                    crate::core::chat_loop::ChatEvent::ToolUsed(tool) => {
                        self.chat_history
                            .push(("system".into(), format!("🔧 Using tool: {}", tool)));
                    }
                    crate::core::chat_loop::ChatEvent::Error(err) => {
                        self.chat_waiting = false;
                        // "Thinking..." 제거
                        if let Some(last) = self.chat_history.last() {
                            if last.1 == "🤔 Thinking..." {
                                self.chat_history.pop();
                            }
                        }
                        self.chat_history
                            .push(("system".into(), format!("⚠️ {}", err)));
                    }
                }
            }
        }
    }

    // === 화면별 키 처리 ===

    fn handle_boot_key(&mut self, key: KeyEvent) {
        // ESC로 부트 시퀀스 스킵
        if key.code == KeyCode::Esc {
            if self.app_config.llm_provider.is_none() {
                self.screen = Screen::Onboard;
            } else {
                self.screen = Screen::Dashboard;
            }
        }
    }

    /// [v1.1.0] Settings 화면 키 처리
    fn handle_settings_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.screen = Screen::Dashboard,
            KeyCode::Up => {
                if self.settings_index > 0 {
                    self.settings_index -= 1;
                }
            }
            KeyCode::Down => {
                if self.settings_index < 3 {
                    self.settings_index += 1;
                }
            }
            KeyCode::Enter => {
                // 선택된 항목 편집 → 온보딩 화면으로 전환하고 해당 필드에 포커스
                match self.settings_index {
                    0 => { /* Provider — 온보딩에서 ←→ 키로 프리셋 변경 */ }
                    1 => self.onboard_field = OnboardField::ApiKey,
                    2 => self.onboard_field = OnboardField::Model,
                    3 => self.onboard_field = OnboardField::TelegramToken,
                    _ => {}
                }
                self.screen = Screen::Onboard;
            }
            _ => {}
        }
    }

    /// [v1.1.0] EditAgent 화면 키 처리
    fn handle_edit_agent_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Esc => self.screen = Screen::Dashboard,
            KeyCode::Up => {
                if self.edit_agent_index > 0 {
                    self.edit_agent_index -= 1;
                }
            }
            KeyCode::Down => {
                let max = self.app_config.agents.len().saturating_sub(1);
                if self.edit_agent_index < max {
                    self.edit_agent_index += 1;
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                // 에이전트 삭제 (기본 #1은 불가)
                let agents = &self.app_config.agents;
                if self.edit_agent_index < agents.len() {
                    let target_id = agents[self.edit_agent_index].id;
                    if target_id == 1 {
                        self.feed_lines.push(format!(
                            "[{}] 기본 에이전트 #1은 삭제할 수 없습니다.",
                            timestamp()
                        ));
                    } else if let Ok(()) = self.app_config.remove_agent(target_id) {
                        self.feed_lines.push(format!(
                            "[{}] 에이전트 #{} 삭제 완료",
                            timestamp(),
                            target_id
                        ));
                        if self.edit_agent_index > 0 {
                            self.edit_agent_index -= 1;
                        }
                    }
                }
            }
            KeyCode::Enter => {
                // 선택된 에이전트의 LLM 설정 변경 → 온보딩 화면으로
                if self.edit_agent_index < self.app_config.agents.len() {
                    let target_id = self.app_config.agents[self.edit_agent_index].id;
                    let _ = self.app_config.switch_agent(target_id);
                    self.screen = Screen::Onboard;
                }
            }
            _ => {}
        }
    }

    fn handle_onboard_key(&mut self, key: KeyEvent) {
        match key.code {
            // [v1.1.0] Esc — 대시보드로 복귀 (온보딩 중단)
            KeyCode::Esc => self.screen = Screen::Dashboard,
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
        // [v0.7.0] 채팅 모드 분기 — 비동기 ChatWorker 기반
        if self.chat_mode {
            match key.code {
                KeyCode::Esc => {
                    self.chat_mode = false;
                }
                KeyCode::Enter => {
                    // [v0.7.0] 대기 중이면 입력 무시
                    if self.chat_waiting {
                        return;
                    }
                    if !self.chat_input.is_empty() {
                        let user_msg = self.chat_input.clone();
                        self.chat_input.clear();
                        self.chat_history.push(("user".into(), user_msg.clone()));

                        // [v0.7.0] worker.send() — 비동기 전송, 즉시 반환
                        if let Some(ref worker) = self.chat_worker {
                            worker.send(&user_msg);
                            // 응답은 tick()의 ChatEvent::Reply에서 자동 수신
                        } else {
                            self.chat_history.push((
                                "system".into(),
                                "⚠️ No LLM configured. Complete onboarding first.".into(),
                            ));
                        }
                    }
                }
                KeyCode::Backspace => {
                    if !self.chat_waiting {
                        self.chat_input.pop();
                    }
                }
                KeyCode::Char(c) => {
                    if !self.chat_waiting && self.chat_input.len() < 500 {
                        self.chat_input.push(c);
                    }
                }
                _ => {}
            }
            return;
        }

        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.running = false,
            // [v0.7.0] 'c' — 채팅 모드 진입 (비동기 ChatWorker)
            KeyCode::Char('c') => {
                self.chat_mode = true;
                // 최초 진입 시 ChatWorker 생성 (background thread)
                if self.chat_worker.is_none() {
                    if let Some(ref llm) = self.app_config.llm_provider {
                        let persona = crate::core::persona::Persona::load(&self.paths.workspace)
                            .unwrap_or_else(|| {
                                crate::core::persona::Persona::new_default(
                                    &self.app_config.agent_name,
                                )
                            });
                        self.chat_worker = Some(crate::core::chat_loop::ChatWorker::spawn(
                            llm,
                            &persona,
                            &self.paths.workspace,
                            Some(self.paths.db_file.clone()),
                        ));
                        self.feed_lines
                            .push(format!("[{}] Chat session started (async).", timestamp()));
                    }
                }
            }
            KeyCode::Char('1') => {
                // [v0.2.0] 에이전트 상태 표시
                self.feed_lines
                    .push(format!("[{}] {}", timestamp(), msg!("dash.agent_status")));
                // [v1.0.0] 활성 에이전트 ID 표시
                self.feed_lines.push(format!(
                    "  Agent #{} - {}",
                    self.app_config.active_agent_id, self.app_config.agent_name
                ));
                if let Some(ref llm) = self.app_config.llm_provider {
                    self.feed_lines.push(format!(
                        "  {}",
                        msg!("dash.model", llm.model, format!("{:?}", llm.preset))
                    ));
                }
                self.feed_lines.push(format!("  {}", msg!("dash.security")));
                self.feed_lines.push(format!(
                    "  [Tab] 에이전트 전환 (1~{})",
                    self.app_config.agents.len()
                ));
            }
            KeyCode::Tab => {
                // [v1.0.0] 에이전트 순환 전환
                let ids: Vec<u8> = self.app_config.agents.iter().map(|a| a.id).collect();
                if ids.len() > 1 {
                    let current = self.app_config.active_agent_id;
                    let idx = ids.iter().position(|&id| id == current).unwrap_or(0);
                    let next_id = ids[(idx + 1) % ids.len()];
                    if let Ok(()) = self.app_config.switch_agent(next_id) {
                        // ChatWorker 재생성
                        self.chat_worker = None;
                        self.chat_history.clear();

                        if let (Some(ref llm), Some(ref mgr)) =
                            (&self.app_config.llm_provider, &self.agent_manager)
                        {
                            if let Some(agent_paths) = mgr.get_paths(next_id) {
                                let persona =
                                    crate::core::persona::Persona::load(&agent_paths.workspace)
                                        .unwrap_or_else(|| {
                                            crate::core::persona::Persona::new_default(
                                                &self.app_config.agent_name,
                                            )
                                        });
                                self.chat_worker = Some(crate::core::chat_loop::ChatWorker::spawn(
                                    llm,
                                    &persona,
                                    &agent_paths.workspace,
                                    Some(agent_paths.db_file.clone()),
                                ));
                            }
                        }

                        self.feed_lines.push(format!(
                            "[{}] 에이전트 #{} ({}) 전환 완료",
                            timestamp(),
                            next_id,
                            self.app_config
                                .active_agent()
                                .map(|a| a.name.as_str())
                                .unwrap_or("?")
                        ));
                    }
                } else {
                    self.feed_lines.push(format!(
                        "[{}] 에이전트가 1개만 등록되어 있어 전환할 수 없습니다.",
                        timestamp()
                    ));
                }
            }
            KeyCode::Char('o') | KeyCode::Char('O') => {
                // [v1.1.0] 온보딩 화면으로 전환
                self.screen = Screen::Onboard;
            }
            KeyCode::Char('s') | KeyCode::Char('S') => {
                // [v1.1.0] Settings 화면으로 전환
                self.settings_index = 0;
                self.screen = Screen::Settings;
            }
            KeyCode::Char('e') | KeyCode::Char('E') => {
                // [v1.1.0] EditAgent 화면으로 전환
                self.edit_agent_index = 0;
                self.screen = Screen::EditAgent;
            }
            KeyCode::Char('+') => {
                // [v1.1.0] 에이전트 추가 + 현재 LLM 복제
                let agent_name = format!("agent-{}", self.app_config.agents.len() + 1);
                match self.app_config.add_agent(&agent_name) {
                    Ok(new_id) => {
                        // 현재 LLM 설정 복제
                        if let Some(ref llm) = self.app_config.llm_provider {
                            if let Some(agent) =
                                self.app_config.agents.iter_mut().find(|a| a.id == new_id)
                            {
                                agent.llm_provider = Some(llm.clone());
                            }
                        }
                        self.feed_lines.push(format!(
                            "[{}] 에이전트 #{} '{}' 추가 완료 (LLM 복제됨)",
                            timestamp(),
                            new_id,
                            agent_name
                        ));
                    }
                    Err(e) => {
                        self.feed_lines.push(format!("[{}] {}", timestamp(), e));
                    }
                }
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
                        .push(format!("[{}] {}", timestamp(), msg!("dash.llm_none")));
                }
            }
            KeyCode::Char('3') => {
                // [v0.2.0] 스킬 목록 표시 (TOML + Rhai 하이브리드)
                self.feed_lines
                    .push(format!("[{}] {}", timestamp(), msg!("dash.skill_header")));
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
                            self.feed_lines.push(format!(
                                "  {}",
                                msg!("dash.skill_builtin", tag, s.name, s.description)
                            ));
                        }
                    }
                    Err(e) => self
                        .feed_lines
                        .push(format!("  {}", msg!("dash.skill_core_fail", e))),
                }
                match crate::skills::load_skills_from_dir(&user_dir, false) {
                    Ok(user_skills) => {
                        for s in &user_skills {
                            let tag = match s.skill_type {
                                crate::skills::SkillType::Static => "TOML",
                                crate::skills::SkillType::Dynamic => "Rhai",
                            };
                            self.feed_lines.push(format!(
                                "  {}",
                                msg!("dash.skill_user", tag, s.name, s.description)
                            ));
                        }
                    }
                    Err(e) => self
                        .feed_lines
                        .push(format!("  {}", msg!("dash.skill_user_fail", e))),
                }
            }
            KeyCode::Char('4') => {
                // [v0.2.0] 타임머신 — DB에서 최근 10건 조회
                self.feed_lines.push(format!(
                    "[{}] {}",
                    timestamp(),
                    msg!("dash.timemachine_header")
                ));
                let cols = msg!("dash.timemachine_cols");
                let col_parts: Vec<&str> = cols.split('|').collect();
                self.feed_lines.push(format!(
                    "  {:>4} | {:>6} | {:>6} | {:<30} | {}",
                    "#",
                    col_parts.first().unwrap_or(&"Type"),
                    col_parts.get(1).unwrap_or(&"Status"),
                    col_parts.get(2).unwrap_or(&"Summary"),
                    col_parts.get(3).unwrap_or(&"Time")
                ));
                self.feed_lines.push(format!("  {}", "─".repeat(70)));
                let db_path = self.paths.db_dir.join("femto_state.db");
                match crate::db::FemtoDb::open(&db_path) {
                    Ok(db) => match db.actions_paged(0, 10) {
                        Ok(records) => {
                            if records.is_empty() {
                                self.feed_lines
                                    .push(format!("  {}", msg!("dash.no_records")));
                            }
                            for r in &records {
                                let status = if r.undone { "↩ Undo" } else { "✅" };
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
                                    self.feed_lines
                                        .push(format!("  {}", msg!("dash.total_count", count)));
                                }
                                Err(_) => {}
                            }
                        }
                        Err(e) => {
                            self.feed_lines
                                .push(format!("  {}", msg!("dash.db_query_fail", e)));
                        }
                    },
                    Err(e) => {
                        self.feed_lines
                            .push(format!("  {}", msg!("dash.db_open_fail", e)));
                    }
                }
            }
            KeyCode::Char('5') => {
                // [v0.3.0] 에이전트 전환
                self.feed_lines.push(format!(
                    "[{}] {}",
                    timestamp(),
                    msg!("dash.agent_switch_header")
                ));
                if self.app_config.agents.is_empty() {
                    self.feed_lines
                        .push(format!("  {}", msg!("dash.no_agents")));
                } else {
                    for a in &self.app_config.agents {
                        let marker = if a.id == self.app_config.active_agent_id {
                            "▶"
                        } else {
                            " "
                        };
                        let status = if a.active {
                            msg!("dash.active")
                        } else {
                            msg!("dash.inactive")
                        };
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
                            self.feed_lines.push(format!(
                                "  {}",
                                msg!("dash.agent_switched", next, agent_name)
                            ));
                        }
                    } else {
                        self.feed_lines
                            .push(format!("  {}", msg!("dash.no_switch")));
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
                            "[{}] {}",
                            timestamp(),
                            msg!("dash.agent_added", id, next_name)
                        ));
                    }
                    Err(e) => {
                        self.feed_lines.push(format!(
                            "[{}] {}",
                            timestamp(),
                            msg!("dash.agent_add_fail", e)
                        ));
                    }
                }
            }
            _ => {}
        }
    }

    // [v1.1.0] 비밀번호 로직 삭제됨 — boot 시 직행

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

        // [v1.1.0] 비밀번호 제거 — 고정 키로 설정 저장
        match config::save_config(
            &self.app_config,
            b"femtoclaw-default-key",
            &self.paths.config_enc,
        ) {
            Ok(_) => {
                self.feed_lines
                    .push(format!("[{}] {}", timestamp(), msg!("onboard.save_ok")));
                self.screen = Screen::Dashboard;
            }
            Err(e) => {
                self.feed_lines.push(format!(
                    "[{}] {}",
                    timestamp(),
                    msg!("onboard.save_fail", e)
                ));
            }
        }
    }

    // === 화면 렌더링 ===

    /// 현재 화면에 맞는 렌더링 함수를 호출한다.
    pub fn render(&self, frame: &mut Frame) {
        // 전체 배경 색상 설정
        let bg = Block::default().style(ratatui::style::Style::default().bg(theme::BACKGROUND));
        frame.render_widget(bg, frame.area());

        match self.screen {
            Screen::Boot => self.render_boot(frame),
            Screen::Onboard => self.render_onboard(frame),
            Screen::Dashboard => self.render_dashboard(frame),
            Screen::Settings => self.render_settings(frame),
            Screen::EditAgent => self.render_edit_agent(frame),
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

    // --- [v1.1.0] Settings 화면 ---
    fn render_settings(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        let title = Paragraph::new("femtoClaw — Settings")
            .style(theme::title())
            .block(Block::bordered().border_style(theme::border()));
        frame.render_widget(title, chunks[0]);

        let provider_name = self
            .app_config
            .llm_provider
            .as_ref()
            .map(|l| format!("{:?}", l.preset))
            .unwrap_or_else(|| "(미설정)".to_string());
        let api_key_display = self
            .app_config
            .llm_provider
            .as_ref()
            .map(|l| {
                if l.api_key.len() > 8 {
                    format!("{}...***", &l.api_key[..8])
                } else {
                    "***".to_string()
                }
            })
            .unwrap_or_else(|| "(미설정)".to_string());
        let model_name = self
            .app_config
            .llm_provider
            .as_ref()
            .map(|l| l.model.clone())
            .unwrap_or_else(|| "(미설정)".to_string());
        let tg_status = self
            .app_config
            .telegram
            .as_ref()
            .map(|t| {
                if t.verified {
                    "✅ Paired".to_string()
                } else {
                    "⚠️ Not verified".to_string()
                }
            })
            .unwrap_or_else(|| "(미설정)".to_string());

        let items = [
            format!("Provider : {}", provider_name),
            format!("API Key  : {}", api_key_display),
            format!("Model    : {}", model_name),
            format!("Telegram : {}", tg_status),
        ];

        let mut lines = vec![Line::from("")];
        for (i, item) in items.iter().enumerate() {
            let marker = if i == self.settings_index {
                "▶ "
            } else {
                "  "
            };
            let style = if i == self.settings_index {
                theme::active_border()
            } else {
                theme::text()
            };
            lines.push(Line::from(Span::styled(
                format!("{}{}", marker, item),
                style,
            )));
        }

        let inner = center_rect(chunks[1], 50, 8);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme::active_border());
        frame.render_widget(
            Paragraph::new(lines).block(block).style(theme::text()),
            inner,
        );

        let footer = Paragraph::new("[↑↓] Select  [Enter] Edit  [Esc] Back").style(theme::muted());
        frame.render_widget(footer, chunks[2]);
    }

    // --- [v1.1.0] EditAgent 화면 ---
    fn render_edit_agent(&self, frame: &mut Frame) {
        let area = frame.area();
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);

        let title = Paragraph::new("femtoClaw — Edit Agent")
            .style(theme::title())
            .block(Block::bordered().border_style(theme::border()));
        frame.render_widget(title, chunks[0]);

        let mut lines = vec![Line::from("")];
        for (i, agent) in self.app_config.agents.iter().enumerate() {
            let marker = if i == self.edit_agent_index {
                "▶ "
            } else {
                "  "
            };
            let active_tag = if agent.id == self.app_config.active_agent_id {
                " (Active)"
            } else {
                ""
            };
            let llm_info = agent
                .llm_provider
                .as_ref()
                .map(|l| format!("{:?}/{}", l.preset, l.model))
                .unwrap_or_else(|| "No LLM".to_string());
            let style = if i == self.edit_agent_index {
                theme::active_border()
            } else {
                theme::text()
            };
            lines.push(Line::from(Span::styled(
                format!(
                    "{}Agent #{}: {}{} — {}",
                    marker, agent.id, agent.name, active_tag, llm_info
                ),
                style,
            )));
        }

        let height = (self.app_config.agents.len() + 3).min(12) as u16;
        let inner = center_rect(chunks[1], 60, height);
        let block = Block::bordered()
            .border_type(BorderType::Rounded)
            .border_style(theme::active_border());
        frame.render_widget(
            Paragraph::new(lines).block(block).style(theme::text()),
            inner,
        );

        let footer = Paragraph::new("[↑↓] Select  [Enter] Edit LLM  [D] Delete  [Esc] Back")
            .style(theme::muted());
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
            ValidationStatus::None => Line::from(Span::styled(
                format!("  Status: [—] {}", msg!("onboard.llm_status_wait")),
                theme::muted(),
            )),
            ValidationStatus::Testing => Line::from(Span::styled(
                format!(
                    "  Status: [⚙ TESTING...] {}",
                    msg!("onboard.llm_status_testing")
                ),
                theme::testing(),
            )),
            ValidationStatus::Ok => Line::from(Span::styled(
                "  Status: [✓ OK] 200 OK — Ready to save",
                theme::success(),
            )),
            ValidationStatus::Failed(e) => Line::from(Span::styled(
                format!(
                    "  Status: [✗ FAIL] {} — {}",
                    e,
                    msg!("onboard.llm_status_fail_retry")
                ),
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
                format!("  Status: [—] {}", msg!("onboard.tg_status_wait")),
                theme::muted(),
            )),
            ValidationStatus::Testing => Line::from(Span::styled(
                format!(
                    "  Status: [⚙ TESTING...] {}",
                    msg!("onboard.tg_status_testing")
                ),
                theme::testing(),
            )),
            ValidationStatus::Ok => Line::from(Span::styled(
                format!("  Status: [✓ OK] {}", msg!("onboard.tg_status_ok")),
                theme::success(),
            )),
            ValidationStatus::Failed(e) => Line::from(Span::styled(
                format!(
                    "  Status: [✗ FAIL] {} — {}",
                    e,
                    msg!("onboard.tg_status_fail_retry")
                ),
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
    // [v0.6.0] 채팅 패널 추가 — 우측을 상단(TERMINAL)+하단(CHAT)으로 분할
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

        // [v0.7.0] 토큰 사용량 표시 (thread-safe)
        let token_info = if let Some(ref worker) = self.chat_worker {
            let ts = worker.token_state();
            format!(
                " │ Tokens: {}/{} ({:.0}%)",
                ts.total,
                ts.max,
                ts.utilization() * 100.0
            )
        } else {
            String::new()
        };

        let header = Paragraph::new(Line::from(vec![
            Span::styled(" femtoClaw Dashboard ", theme::status_bar()),
            Span::styled(format!(" │ Model: {} ", provider_name), theme::title()),
            Span::styled(
                format!(" │ Status: [SECURE] │ Jailed: [ON]{} ", token_info),
                theme::text(),
            ),
        ]));
        frame.render_widget(header, outer[0]);

        // 메인 — 좌(시스템)/우(터미널+채팅) 분할
        let main = Layout::horizontal([Constraint::Length(28), Constraint::Min(0)]).split(outer[1]);

        // 좌측: 시스템 정보 + 메뉴
        let chat_indicator = if self.chat_mode { " ◀ ACTIVE" } else { "" };
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
            Line::from(Span::styled(
                format!("  [C] Chat{}", chat_indicator),
                if self.chat_mode {
                    theme::title()
                } else {
                    theme::text()
                },
            )),
        ];
        let sys_block = Block::bordered()
            .title(Span::styled("─ SYSTEM ─", theme::title()))
            .border_style(theme::border());
        let sys_widget = Paragraph::new(sys_lines).block(sys_block);
        frame.render_widget(sys_widget, main[0]);

        // 우측: 터미널 + 채팅 패널 분할
        if self.chat_mode {
            // [v0.6.0] 채팅 모드: 상단(대화 기록) + 하단(입력)
            let right = Layout::vertical([
                Constraint::Min(0),    // 대화 기록
                Constraint::Length(3), // 입력 영역
            ])
            .split(main[1]);

            // 대화 기록 렌더링
            let visible = right[0].height.saturating_sub(2) as usize;
            let start = self.chat_history.len().saturating_sub(visible);
            let chat_lines: Vec<Line> = self.chat_history[start..]
                .iter()
                .map(|(role, content)| {
                    let (prefix, style) = match role.as_str() {
                        "user" => ("You: ", theme::title()),
                        "assistant" => ("🐾 : ", theme::text()),
                        _ => ("sys: ", theme::muted()),
                    };
                    // 긴 메시지는 줄바꿈 없이 잘라서 표시
                    let display: String = content.chars().take(200).collect();
                    Line::from(Span::styled(format!("{}{}", prefix, display), style))
                })
                .collect();

            let chat_block = Block::bordered()
                .title(Span::styled("─ CHAT ─", theme::title()))
                .border_style(theme::active_border());
            let chat_widget = Paragraph::new(chat_lines)
                .block(chat_block)
                .wrap(Wrap { trim: false });
            frame.render_widget(chat_widget, right[0]);

            // 입력 영역
            let input_text = format!("▶ {}_", self.chat_input);
            let input_block = Block::bordered()
                .title(Span::styled("─ INPUT ─", theme::title()))
                .border_style(theme::active_border());
            let input_widget =
                Paragraph::new(Span::styled(input_text, theme::input())).block(input_block);
            frame.render_widget(input_widget, right[1]);
        } else {
            // 기존 터미널 피드 (채팅 비활성 시)
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
        }

        // 상태바
        let status_text = if self.chat_mode {
            " [Esc] Back  [Enter] Send  │ Chat Mode Active"
        } else {
            " [Q] Quit  [1-5] Menu  [A] Add Agent  [C] Chat  [U] Undo"
        };
        let status = Paragraph::new(status_text).style(theme::muted());
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
        .map_err(|e| format!("{}", msg!("err.http_client", e)))?;

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
                Err(msg!("val.timeout").to_string())
            } else if e.is_connect() {
                Err(msg!("val.connect_fail").to_string())
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
        Ok(resp) if resp.status().is_success() => Ok(msg!("val.bot_confirmed").to_string()),
        Ok(resp) => Err(format!(
            "HTTP {} — {}",
            resp.status(),
            msg!("val.check_token")
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

/// [v0.7.0] CJK 보정된 입력 텍스트 자르기
/// unicode-width로 표시 너비를 정확히 계산하여 CJK 문자(2칸)도 올바르게 처리.
/// max_width: 보여줄 최대 칸 수(터미널 열). 초과 시 앞에 "…"를 붙여 tail 표시.
fn truncate_for_display(text: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthStr;

    let text_width = UnicodeWidthStr::width(text);
    if text_width <= max_width {
        return text.to_string();
    }

    // 끝에서부터 max_width - 1 칸까지 문자를 수집 ("…" 1칸 예약)
    let target = max_width.saturating_sub(1);
    let mut result = String::new();
    let mut collected_width = 0;

    // 뒤에서부터 수집
    let chars: Vec<char> = text.chars().collect();
    for &ch in chars.iter().rev() {
        let w = unicode_width::UnicodeWidthChar::width(ch).unwrap_or(0);
        if collected_width + w > target {
            break;
        }
        collected_width += w;
        result.push(ch);
    }

    // 역순 수집이므로 다시 뒤집기
    let tail: String = result.chars().rev().collect();
    format!("…{}", tail)
}
