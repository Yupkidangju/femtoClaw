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
// [v0.1.0] 추가 기능:
//   - Exponential Backoff (1→2→4→...→60초)
//   - 오프라인 큐잉 (네트워크 끊김 시 메시지 대기열)
//   - Graceful Shutdown (shutdown_token으로 봇 종료)
//
// 통신: std::sync::mpsc 채널로 TUI ↔ 봇 스레드 간 메시지 교환

use std::sync::{Arc, Mutex, mpsc};
use std::sync::atomic::{AtomicBool, Ordering};
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
    /// 봇 종료됨
    Shutdown,
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

/// [v0.1.0] 오프라인 큐 — 네트워크 끊김 시 메시지를 대기열에 저장
#[derive(Debug)]
pub struct OfflineQueue {
    /// 미전송 메시지 대기열
    messages: Vec<String>,
    /// 최대 큐 크기 (메모리 보호)
    max_size: usize,
}

impl OfflineQueue {
    pub fn new(max_size: usize) -> Self {
        OfflineQueue { messages: Vec::new(), max_size }
    }

    /// 메시지를 큐에 추가 (최대 크기 초과 시 가장 오래된 것 제거)
    pub fn enqueue(&mut self, msg: String) {
        if self.messages.len() >= self.max_size {
            self.messages.remove(0);
        }
        self.messages.push(msg);
    }

    /// 대기열의 모든 메시지를 꺼낸다 (FIFO)
    pub fn drain(&mut self) -> Vec<String> {
        std::mem::take(&mut self.messages)
    }

    pub fn len(&self) -> usize {
        self.messages.len()
    }

    pub fn is_empty(&self) -> bool {
        self.messages.is_empty()
    }
}

/// [v0.1.0] Exponential Backoff 계산기
/// 실패 시 1→2→4→8→...→60초까지 대기 시간 증가.
/// 5분(300초) 연속 실패 시 경고 플래그 활성화.
pub struct Backoff {
    /// 현재 대기 시간 (초)
    current_secs: u64,
    /// 최대 대기 시간 (초)
    max_secs: u64,
    /// 연속 실패 총 시간 (초)
    total_fail_secs: u64,
    /// 5분 경고 발동 여부
    pub warning_triggered: bool,
}

impl Backoff {
    pub fn new() -> Self {
        Backoff {
            current_secs: 1,
            max_secs: 60,
            total_fail_secs: 0,
            warning_triggered: false,
        }
    }

    /// 다음 대기 시간을 반환하고 상태를 갱신한다
    pub fn next_delay(&mut self) -> std::time::Duration {
        let delay = std::time::Duration::from_secs(self.current_secs);
        self.total_fail_secs += self.current_secs;

        // 5분(300초) 연속 실패 시 경고
        if self.total_fail_secs >= 300 && !self.warning_triggered {
            self.warning_triggered = true;
        }

        // Exponential 증가 (최대 max_secs)
        self.current_secs = (self.current_secs * 2).min(self.max_secs);
        delay
    }

    /// 성공 시 리셋
    pub fn reset(&mut self) {
        self.current_secs = 1;
        self.total_fail_secs = 0;
        self.warning_triggered = false;
    }
}

/// 6자리 랜덤 PIN 생성
pub fn generate_pin() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    format!("{:06}", rng.gen_range(0..999999))
}

/// [v0.1.0] Graceful Shutdown 을 위한 종료 플래그
/// AtomicBool로 스레드 간 안전하게 공유.
pub fn create_shutdown_flag() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

/// [v0.1.0] 텔레그램 봇을 별도 스레드에서 시작한다.
/// TUI 메인 루프를 블로킹하지 않음.
///
/// 반환: (이벤트 수신 채널, 명령 송신 채널, PIN 코드, 종료 플래그)
pub fn spawn_bot(
    token: String,
    shutdown_flag: Arc<AtomicBool>,
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
            run_bot(token, pin_clone, event_tx, cmd_rx, shutdown_flag).await;
        });
    });

    (event_rx, cmd_tx, pin)
}

/// 봇 메인 루프 (tokio async) — Backoff 포함
async fn run_bot(
    token: String,
    pin: String,
    event_tx: mpsc::Sender<BotEvent>,
    _cmd_rx: mpsc::Receiver<BotCommand>,
    shutdown_flag: Arc<AtomicBool>,
) {
    let mut backoff = Backoff::new();

    loop {
        // Graceful Shutdown 체크
        if shutdown_flag.load(Ordering::Relaxed) {
            let _ = event_tx.send(BotEvent::Shutdown);
            break;
        }

        let bot = Bot::new(&token);
        let state = Arc::new(Mutex::new(BotState::new(pin.clone())));
        let tx = event_tx.clone();
        let flag = shutdown_flag.clone();

        // teloxide dispatcher: 모든 메시지를 핸들러로 라우팅
        let handler = Update::filter_message()
            .endpoint(move |bot: Bot, msg: Message, state: Arc<Mutex<BotState>>, tx: mpsc::Sender<BotEvent>| {
                handle_message(bot, msg, state, tx)
            });

        let mut dispatcher = Dispatcher::builder(bot, handler)
            .dependencies(dptree::deps![state, tx])
            .default_handler(|_upd| async {})
            .error_handler(LoggingErrorHandler::with_custom_text("텔레그램 디스패처 오류"))
            .build();

        // shutdown_token으로 외부에서 디스패처 종료 가능
        let shutdown_token = dispatcher.shutdown_token();

        // 종료 감시 태스크
        let flag_clone = flag.clone();
        tokio::spawn(async move {
            loop {
                tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                if flag_clone.load(Ordering::Relaxed) {
                    shutdown_token.shutdown().ok();
                    break;
                }
            }
        });

        // 디스패처 실행 (연결 끊기면 여기서 반환)
        dispatcher.dispatch().await;

        // 종료 플래그 확인 — 의도적 종료면 루프 탈출
        if shutdown_flag.load(Ordering::Relaxed) {
            let _ = event_tx.send(BotEvent::Shutdown);
            break;
        }

        // 비정상 종료 → Exponential Backoff 후 재연결
        let delay = backoff.next_delay();
        let _ = event_tx.send(BotEvent::Error(
            format!("텔레그램 연결 끊김 — {}초 후 재연결", delay.as_secs())
        ));

        if backoff.warning_triggered {
            let _ = event_tx.send(BotEvent::Error(
                "⚠ 5분 이상 연속 실패! 네트워크 및 토큰 확인 필요".to_string()
            ));
        }

        tokio::time::sleep(delay).await;
    }
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
                        "✅ *femtoClaw 페어링 성공\\!*\n\n\
                        기기가 연결되었습니다\\.\n\
                        이제 메시지를 보내면 에이전트가 응답합니다\\.\n\n\
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
            bot.send_message(chat_id, "⏳ 처리 중...")
                .await.ok();
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_generation() {
        let pin = generate_pin();
        assert_eq!(pin.len(), 6);
        assert!(pin.chars().all(|c| c.is_ascii_digit()));
    }

    #[test]
    fn test_backoff_exponential() {
        let mut backoff = Backoff::new();
        // 1 → 2 → 4 → 8 → 16 → 32 → 60(최대)
        assert_eq!(backoff.next_delay().as_secs(), 1);
        assert_eq!(backoff.next_delay().as_secs(), 2);
        assert_eq!(backoff.next_delay().as_secs(), 4);
        assert_eq!(backoff.next_delay().as_secs(), 8);
        assert_eq!(backoff.next_delay().as_secs(), 16);
        assert_eq!(backoff.next_delay().as_secs(), 32);
        assert_eq!(backoff.next_delay().as_secs(), 60); // 최대값 도달
        assert_eq!(backoff.next_delay().as_secs(), 60); // 최대값 유지
    }

    #[test]
    fn test_backoff_reset() {
        let mut backoff = Backoff::new();
        backoff.next_delay(); // 1
        backoff.next_delay(); // 2
        backoff.reset();
        assert_eq!(backoff.next_delay().as_secs(), 1); // 리셋 후 다시 1부터
    }

    #[test]
    fn test_backoff_5min_warning() {
        let mut backoff = Backoff::new();
        // 1+2+4+8+16+32+60+60+60+60 = 303초 > 300초
        for _ in 0..10 {
            backoff.next_delay();
        }
        assert!(backoff.warning_triggered, "5분 이상 실패 시 경고 발동");
    }

    #[test]
    fn test_offline_queue() {
        let mut queue = OfflineQueue::new(3);
        assert!(queue.is_empty());

        queue.enqueue("msg1".to_string());
        queue.enqueue("msg2".to_string());
        assert_eq!(queue.len(), 2);

        // 최대 크기 초과 시 가장 오래된 것 제거
        queue.enqueue("msg3".to_string());
        queue.enqueue("msg4".to_string());
        assert_eq!(queue.len(), 3); // msg1 제거됨

        let drained = queue.drain();
        assert_eq!(drained, vec!["msg2", "msg3", "msg4"]);
        assert!(queue.is_empty());
    }
}
