// femtoClaw — 텔레그램 봇 엔진
// [v0.1.0] Step 4: teloxide Long-Polling 단일 에이전트 봇.
//
// 동작 흐름:
//   1. TUI/Headless에서 봇 스레드 시작 (별도 tokio 런타임)
//   2. 6자리 랜덤 PIN 생성 → TUI에 표시
//   3. 사용자가 텔레그램에서 /pair PIN 전송
//   4. PIN 매칭 성공 → chat_id 기록, 페어링 완료
//   5. 이후 메시지는 LLM 에이전트로 전달 → 응답 반환
//
// 통신: std::sync::mpsc 채널로 TUI ↔ 봇 스레드 간 메시지 교환

use std::sync::{Arc, Mutex, mpsc};
use teloxide::prelude::*;
use teloxide::types::ParseMode;

/// TUI ← 봇 스레드로 전달되는 이벤트
#[derive(Debug, Clone)]
pub enum BotEvent {
    /// 봇 시작됨 (PIN 코드 포함)
    Started(String),
    /// 페어링 성공 (chat_id, 사용자명)
    Paired(i64, String),
    /// 사용자 메시지 수신
    MessageReceived(String),
    /// 에이전트 응답 전송 완료
    ResponseSent(String),
    /// 오류 발생
    Error(String),
}

/// TUI → 봇 스레드로 전달되는 명령
#[derive(Debug, Clone)]
pub enum BotCommand {
    /// 에이전트 응답을 텔레그램으로 전송
    SendResponse(String),
    /// 봇 종료
    Shutdown,
}

/// 봇 상태 (Thread-safe 공유)
#[derive(Debug, Clone)]
pub struct BotState {
    /// 페어링 PIN (6자리)
    pub pin: String,
    /// 페어링된 chat_id (None이면 미페어링)
    pub paired_chat_id: Option<i64>,
    /// 페어링된 사용자명
    pub paired_username: Option<String>,
}

impl BotState {
    pub fn new(pin: String) -> Self {
        BotState {
            pin,
            paired_chat_id: None,
            paired_username: None,
        }
    }

    pub fn is_paired(&self) -> bool {
        self.paired_chat_id.is_some()
    }
}

/// 6자리 랜덤 PIN 생성
pub fn generate_pin() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..999999))
}

/// [v0.1.0] 텔레그램 봇을 별도 스레드에서 시작한다.
/// TUI 메인 루프를 블로킹하지 않음.
///
/// 반환: (이벤트 수신 채널, 명령 송신 채널, PIN 코드)
pub fn spawn_bot(
    token: String,
) -> (mpsc::Receiver<BotEvent>, mpsc::Sender<BotCommand>, String) {
    let (event_tx, event_rx) = mpsc::channel::<BotEvent>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<BotCommand>();

    let pin = generate_pin();
    let pin_clone = pin.clone();

    // 봇 시작 이벤트 즉시 전송
    let _ = event_tx.send(BotEvent::Started(pin.clone()));

    std::thread::spawn(move || {
        // 봇 전용 tokio 런타임 생성
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio 런타임 생성 실패");

        rt.block_on(async move {
            run_bot(token, pin_clone, event_tx, cmd_rx).await;
        });
    });

    (event_rx, cmd_tx, pin)
}

/// 봇 메인 루프 (tokio async)
async fn run_bot(
    token: String,
    pin: String,
    event_tx: mpsc::Sender<BotEvent>,
    _cmd_rx: mpsc::Receiver<BotCommand>,
) {
    let bot = Bot::new(&token);

    let state = Arc::new(Mutex::new(BotState::new(pin)));
    let tx = event_tx.clone();

    // teloxide dispatcher: 모든 메시지를 핸들러로 라우팅
    let handler = Update::filter_message()
        .endpoint(move |bot: Bot, msg: Message, state: Arc<Mutex<BotState>>, tx: mpsc::Sender<BotEvent>| {
            handle_message(bot, msg, state, tx)
        });

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![state, tx])
        .default_handler(|_upd| async {})
        .error_handler(LoggingErrorHandler::with_custom_text("텔레그램 디스패처 오류"))
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

/// 개별 메시지 처리
async fn handle_message(
    bot: Bot,
    msg: Message,
    state: Arc<Mutex<BotState>>,
    event_tx: mpsc::Sender<BotEvent>,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let text = msg.text().unwrap_or("").to_string();
    let chat_id = msg.chat.id;

    // 페어링 상태 확인
    let is_paired = {
        let s = state.lock().unwrap();
        s.is_paired()
    };

    if !is_paired {
        // 미페어링 상태: /pair 명령만 처리
        if text.starts_with("/pair") {
            let parts: Vec<&str> = text.split_whitespace().collect();
            if parts.len() == 2 {
                let input_pin = parts[1];
                let pin_match = {
                    let s = state.lock().unwrap();
                    s.pin == input_pin
                };

                if pin_match {
                    // 페어링 성공
                    let username = msg.from
                        .as_ref()
                        .and_then(|u| u.username.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    {
                        let mut s = state.lock().unwrap();
                        s.paired_chat_id = Some(chat_id.0);
                        s.paired_username = Some(username.clone());
                    }

                    bot.send_message(chat_id, format!(
                        "✅ *femtoClaw 페어링 성공!*\n\n\
                        기기가 연결되었습니다.\n\
                        이제 메시지를 보내면 에이전트가 응답합니다.\n\n\
                        `/help` — 명령어 목록"
                    ))
                    .parse_mode(ParseMode::MarkdownV2)
                    .await
                    .ok();

                    let _ = event_tx.send(BotEvent::Paired(chat_id.0, username));
                } else {
                    bot.send_message(chat_id, "❌ PIN이 일치하지 않습니다. TUI에 표시된 PIN을 확인하세요.")
                        .await.ok();
                }
            } else {
                bot.send_message(chat_id, "사용법: /pair 123456")
                    .await.ok();
            }
        } else {
            bot.send_message(chat_id, "🔒 먼저 페어링이 필요합니다.\n/pair [PIN코드]를 입력하세요.")
                .await.ok();
        }
    } else {
        // 페어링 완료 상태: 메시지를 에이전트로 전달
        let paired_id = {
            let s = state.lock().unwrap();
            s.paired_chat_id.unwrap_or(0)
        };

        // 자신의 chat_id만 허용
        if chat_id.0 != paired_id {
            bot.send_message(chat_id, "⚠️ 이 봇은 다른 기기와 페어링되어 있습니다.")
                .await.ok();
            return Ok(());
        }

        if text.starts_with('/') {
            // 명령어 처리
            match text.as_str() {
                "/help" => {
                    bot.send_message(chat_id,
                        "📋 femtoClaw 명령어:\n\n\
                        /help — 이 도움말\n\
                        /status — 에이전트 상태\n\
                        /undo — 마지막 동작 취소\n\n\
                        그 외 메시지는 에이전트에게 전달됩니다."
                    ).await.ok();
                }
                "/status" => {
                    bot.send_message(chat_id, "🟢 femtoClaw 에이전트 활성 중")
                        .await.ok();
                }
                _ => {
                    bot.send_message(chat_id, "알 수 없는 명령어입니다. /help를 확인하세요.")
                        .await.ok();
                }
            }
        } else {
            // 일반 메시지 → 에이전트로 전달
            let _ = event_tx.send(BotEvent::MessageReceived(text));

            // TODO: 에이전트 응답을 받아서 전송하는 로직
            // 현재는 수신 확인만 전송
            bot.send_message(chat_id, "⏳ 처리 중...")
                .await.ok();
        }
    }

    Ok(())
}
