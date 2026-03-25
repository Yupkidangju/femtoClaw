// femtoClaw — 텔레그램 봇 엔진
// [v0.3.0] Step 4/8c: teloxide Long-Polling + 멀티 에이전트 라우팅.
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

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
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
    /// [v0.3.0] 에이전트 전환됨 (agent_id)
    AgentSwitched(u8),
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
    /// [v0.3.0] 현재 활성 에이전트 ID
    pub active_agent_id: u8,
    /// [v0.3.0] 등록된 에이전트 ID 목록
    pub agent_ids: Vec<u8>,
}

impl BotState {
    pub fn new(pin: String) -> Self {
        BotState {
            pin,
            paired_chat_id: None,
            paired_username: None,
            active_agent_id: 1,
            agent_ids: vec![1],
        }
    }

    /// [v0.4.0] 이전 페어링 정보를 복원하여 생성
    /// config.enc에 저장된 chat_id가 있으면 재시작 시 자동 페어링
    pub fn new_with_paired(pin: String, chat_id: Option<i64>) -> Self {
        BotState {
            pin,
            paired_chat_id: chat_id,
            paired_username: chat_id.map(|id| format!("restored_{}", id)),
            active_agent_id: 1,
            agent_ids: vec![1],
        }
    }

    /// [v0.3.0] 에이전트 목록을 설정
    pub fn set_agents(&mut self, ids: Vec<u8>) {
        self.agent_ids = ids;
        if !self.agent_ids.contains(&self.active_agent_id) {
            self.active_agent_id = *self.agent_ids.first().unwrap_or(&1);
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
        OfflineQueue {
            messages: Vec::new(),
            max_size,
        }
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

/// [v0.4.0] 텔레그램 봇을 별도 스레드에서 시작한다.
/// TUI 메인 루프를 블로킹하지 않음.
/// saved_chat_id: config.enc에 저장된 이전 페어링 chat_id (있으면 자동 복원)
///
/// 반환: (이벤트 수신 채널, 명령 송신 채널, PIN 코드)
pub fn spawn_bot(
    token: String,
    shutdown_flag: Arc<AtomicBool>,
    saved_chat_id: Option<i64>,
) -> (mpsc::Receiver<BotEvent>, mpsc::Sender<BotCommand>, String) {
    let (event_tx, event_rx) = mpsc::channel::<BotEvent>();
    let (cmd_tx, cmd_rx) = mpsc::channel::<BotCommand>();

    let pin = generate_pin();
    let pin_clone = pin.clone();

    // 봇 시작 이벤트 즉시 전송
    let _ = event_tx.send(BotEvent::Started(pin.clone()));

    // [v0.4.0] 이전 페어링 복원 시 알림
    if saved_chat_id.is_some() {
        let _ = event_tx.send(BotEvent::Paired(
            saved_chat_id.unwrap(),
            "restored".to_string(),
        ));
    }

    std::thread::spawn(move || {
        // 봇 전용 tokio 런타임 생성
        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(2)
            .enable_all()
            .build()
            .expect("tokio runtime creation failed");

        rt.block_on(async move {
            run_bot(
                token,
                pin_clone,
                event_tx,
                cmd_rx,
                shutdown_flag,
                saved_chat_id,
            )
            .await;
        });
    });

    (event_rx, cmd_tx, pin)
}

/// 봇 메인 루프 (tokio async) — Backoff 포함
/// [v0.4.0] saved_chat_id: 이전 페어링 복원용
/// [v0.8.0] cmd_rx: 에이전트 응답을 텔레그램으로 전송하는 커맨드 수신 채널
async fn run_bot(
    token: String,
    pin: String,
    event_tx: mpsc::Sender<BotEvent>,
    cmd_rx: mpsc::Receiver<BotCommand>,
    shutdown_flag: Arc<AtomicBool>,
    saved_chat_id: Option<i64>,
) {
    let mut backoff = Backoff::new();

    // [v0.8.0] cmd_rx를 Arc<Mutex>로 감싸서 응답 전송 태스크에서 공유
    let cmd_rx = Arc::new(Mutex::new(cmd_rx));

    loop {
        // Graceful Shutdown 체크
        if shutdown_flag.load(Ordering::Relaxed) {
            let _ = event_tx.send(BotEvent::Shutdown);
            break;
        }

        let bot = Bot::new(&token);
        // [v0.4.0] 이전 페어링 정보가 있으면 복원하여 /pair 생략
        let state = Arc::new(Mutex::new(BotState::new_with_paired(
            pin.clone(),
            saved_chat_id,
        )));
        let tx = event_tx.clone();
        let flag = shutdown_flag.clone();

        // [v0.8.0] 에이전트 응답 전송 태스크 — cmd_rx에서 SendResponse 수신 시 텔레그램으로 전송
        let response_bot = Bot::new(&token);
        let response_state = state.clone();
        let response_cmd_rx = cmd_rx.clone();
        let response_flag = shutdown_flag.clone();
        let response_tx = event_tx.clone();
        tokio::spawn(async move {
            loop {
                if response_flag.load(Ordering::Relaxed) {
                    break;
                }

                // std::sync::mpsc는 blocking이므로 try_recv로 폴링
                let cmd = {
                    response_cmd_rx
                        .lock()
                        .ok()
                        .and_then(|rx| rx.try_recv().ok())
                };

                match cmd {
                    Some(BotCommand::SendResponse(text)) => {
                        // 페어링된 chat_id에 응답 전송
                        let chat_id = { response_state.lock().ok().and_then(|s| s.paired_chat_id) };
                        if let Some(cid) = chat_id {
                            // [v0.9.0] 텔레그램 메시지 길이 제한(4096자)을 위해 4000자 단위로 분할 전송
                            let chars: Vec<char> = text.chars().collect();
                            for chunk in chars.chunks(4000) {
                                let chunk_str: String = chunk.iter().collect();
                                let _ = response_bot
                                    .send_message(teloxide::types::ChatId(cid), &chunk_str)
                                    .await;
                            }
                            let _ = response_tx
                                .send(BotEvent::ResponseSent(text.chars().take(50).collect()));
                        }
                    }
                    Some(BotCommand::Shutdown) => {
                        break;
                    }
                    None => {
                        // 메시지 없음 — 100ms 대기
                        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                    }
                }
            }
        });

        // teloxide dispatcher: 모든 메시지를 핸들러로 라우팅
        let handler = Update::filter_message().endpoint(
            move |bot: Bot,
                  msg: Message,
                  state: Arc<Mutex<BotState>>,
                  tx: mpsc::Sender<BotEvent>| { handle_message(bot, msg, state, tx) },
        );

        let mut dispatcher = Dispatcher::builder(bot, handler)
            .dependencies(dptree::deps![state, tx])
            .default_handler(|_upd| async {})
            .error_handler(LoggingErrorHandler::with_custom_text(
                "Telegram dispatcher error",
            ))
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
        let _ = event_tx.send(BotEvent::Error(format!(
            "Connection lost — reconnecting in {}s",
            delay.as_secs()
        )));

        if backoff.warning_triggered {
            let _ = event_tx.send(BotEvent::Error(
                "⚠ 5+ min failures! Check network and token".to_string(),
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
                    let username = msg
                        .from
                        .as_ref()
                        .and_then(|u| u.username.clone())
                        .unwrap_or_else(|| "Unknown".to_string());

                    {
                        let mut s = state.lock().unwrap();
                        s.paired_chat_id = Some(chat_id.0);
                        s.paired_username = Some(username.clone());
                    }

                    bot.send_message(chat_id, crate::msg!("bot.pair_success"))
                        .await
                        .ok();

                    let _ = event_tx.send(BotEvent::Paired(chat_id.0, username));
                } else {
                    bot.send_message(chat_id, crate::msg!("bot.pair_fail"))
                        .await
                        .ok();
                }
            } else {
                bot.send_message(chat_id, "Usage: /pair 123456").await.ok();
            }
        } else {
            bot.send_message(chat_id, crate::msg!("bot.pair_prompt"))
                .await
                .ok();
        }
    } else {
        // 페어링 완료 상태: 메시지를 에이전트로 전달
        let paired_id = {
            let s = state.lock().unwrap();
            s.paired_chat_id.unwrap_or(0)
        };

        // 자신의 chat_id만 허용
        if chat_id.0 != paired_id {
            bot.send_message(chat_id, "⚠️ This bot is paired with another device.")
                .await
                .ok();
            return Ok(());
        }

        if text.starts_with('/') {
            // 명령어 처리
            match text.as_str() {
                "/help" => {
                    bot.send_message(chat_id, crate::msg!("bot.help"))
                        .await
                        .ok();
                }
                "/status" => {
                    let agent_id = {
                        let s = state.lock().unwrap();
                        s.active_agent_id
                    };
                    bot.send_message(chat_id, format!("🟢 femtoClaw Agent #{} active", agent_id))
                        .await
                        .ok();
                }
                // [v0.3.0] /agents — 에이전트 목록
                "/agents" => {
                    let (agents, active) = {
                        let s = state.lock().unwrap();
                        (s.agent_ids.clone(), s.active_agent_id)
                    };
                    let list: Vec<String> = agents
                        .iter()
                        .map(|id| {
                            if *id == active {
                                format!("▶ Agent #{} (active)", id)
                            } else {
                                format!("  Agent #{}", id)
                            }
                        })
                        .collect();
                    bot.send_message(chat_id, format!("👥 Agent list:\n{}", list.join("\n")))
                        .await
                        .ok();
                }
                _ => {
                    // [v0.3.0] /agent N 명령어 처리
                    if text.starts_with("/agent ") {
                        let parts: Vec<&str> = text.split_whitespace().collect();
                        if parts.len() == 2 {
                            if let Ok(agent_id) = parts[1].parse::<u8>() {
                                let success = {
                                    let mut s = state.lock().unwrap();
                                    if s.agent_ids.contains(&agent_id) {
                                        s.active_agent_id = agent_id;
                                        true
                                    } else {
                                        false
                                    }
                                };
                                if success {
                                    bot.send_message(
                                        chat_id,
                                        format!("✅ Switched to Agent #{}", agent_id),
                                    )
                                    .await
                                    .ok();
                                    let _ = event_tx.send(BotEvent::AgentSwitched(agent_id));
                                } else {
                                    bot.send_message(
                                        chat_id,
                                        format!("❌ Agent #{} not found. Check /agents.", agent_id),
                                    )
                                    .await
                                    .ok();
                                }
                            } else {
                                bot.send_message(chat_id, "Usage: /agent 1").await.ok();
                            }
                        }
                    } else {
                        bot.send_message(chat_id, "Unknown command. Try /help.")
                            .await
                            .ok();
                    }
                }
            }
        } else {
            // 일반 메시지 → 에이전트로 전달
            let _ = event_tx.send(BotEvent::MessageReceived(text));
            bot.send_message(chat_id, "⏳ Processing...").await.ok();
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
