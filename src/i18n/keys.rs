// femtoClaw — i18n 메시지 키 상수
// [v0.5.0] 모든 다국어 메시지 키를 상수로 중앙 관리한다.
//
// 네이밍 규칙:
//   - 모듈.기능.세부 (예: "err.home_not_found", "pw.empty")
//   - 카테고리 접두사: err, pw, onboard, dash, boot, cli, tool, guide, bot, db

// === 에러 메시지 ===
pub const ERR_HOME_NOT_FOUND: &str = "err.home_not_found";
pub const ERR_SANDBOX_CREATE: &str = "err.sandbox_create";
pub const ERR_ALREADY_RUNNING: &str = "err.already_running";
pub const ERR_LOCK_FILE: &str = "err.lock_file";
pub const ERR_KEY_DERIVATION: &str = "err.key_derivation";
pub const ERR_ENCRYPTION: &str = "err.encryption";
pub const ERR_DECRYPTION: &str = "err.decryption";
pub const ERR_CONFIG_IO: &str = "err.config_io";
pub const ERR_INVALID_CONFIG: &str = "err.invalid_config";
pub const ERR_SERIALIZATION: &str = "err.serialization";
pub const ERR_MAX_AGENTS: &str = "err.max_agents";
pub const ERR_HTTP_CLIENT: &str = "err.http_client";

// === 비밀번호 화면 ===
pub const PW_EMPTY: &str = "pw.empty";
pub const PW_TOO_SHORT: &str = "pw.too_short";
pub const PW_MISMATCH: &str = "pw.mismatch";
pub const PW_KEY_GENERATED: &str = "pw.key_generated";
pub const PW_SAVE_FAIL: &str = "pw.save_fail";
pub const PW_DECRYPT_OK: &str = "pw.decrypt_ok";
pub const PW_3FAIL_RESET: &str = "pw.3fail_reset";
pub const PW_WRONG_PW: &str = "pw.wrong_pw";

// === 온보딩 화면 ===
pub const ONBOARD_SAVE_OK: &str = "onboard.save_ok";
pub const ONBOARD_SAVE_FAIL: &str = "onboard.save_fail";
pub const ONBOARD_LLM_WAIT: &str = "onboard.llm_status_wait";
pub const ONBOARD_LLM_TESTING: &str = "onboard.llm_status_testing";
pub const ONBOARD_LLM_FAIL_RETRY: &str = "onboard.llm_status_fail_retry";
pub const ONBOARD_TG_WAIT: &str = "onboard.tg_status_wait";
pub const ONBOARD_TG_TESTING: &str = "onboard.tg_status_testing";
pub const ONBOARD_TG_OK: &str = "onboard.tg_status_ok";
pub const ONBOARD_TG_FAIL_RETRY: &str = "onboard.tg_status_fail_retry";

// === 부트 ===
pub const BOOT_INIT_MSG: &str = "boot.init_msg";

// === 대시보드 피드 ===
pub const FEED_LLM_OK: &str = "feed.llm_verify_ok";
pub const FEED_LLM_OK_SIMPLE: &str = "feed.llm_verify_ok_simple";
pub const FEED_LLM_FAIL: &str = "feed.llm_verify_fail";
pub const FEED_TG_OK: &str = "feed.tg_verify_ok";
pub const FEED_TG_FAIL: &str = "feed.tg_verify_fail";

// === 대시보드 메뉴 ===
pub const DASH_AGENT_STATUS: &str = "dash.agent_status";
pub const DASH_AGENT_NAME: &str = "dash.agent_name";
pub const DASH_MODEL: &str = "dash.model";
pub const DASH_SECURITY: &str = "dash.security";
pub const DASH_LLM_NONE: &str = "dash.llm_none";
pub const DASH_SKILL_HEADER: &str = "dash.skill_header";
pub const DASH_SKILL_BUILTIN: &str = "dash.skill_builtin";
pub const DASH_SKILL_CORE_FAIL: &str = "dash.skill_core_fail";
pub const DASH_SKILL_USER: &str = "dash.skill_user";
pub const DASH_SKILL_USER_FAIL: &str = "dash.skill_user_fail";
pub const DASH_TM_HEADER: &str = "dash.timemachine_header";
pub const DASH_TM_COLS: &str = "dash.timemachine_cols";
pub const DASH_NO_RECORDS: &str = "dash.no_records";
pub const DASH_TOTAL_COUNT: &str = "dash.total_count";
pub const DASH_DB_QUERY_FAIL: &str = "dash.db_query_fail";
pub const DASH_DB_OPEN_FAIL: &str = "dash.db_open_fail";
pub const DASH_AGENT_SWITCH: &str = "dash.agent_switch_header";
pub const DASH_NO_AGENTS: &str = "dash.no_agents";
pub const DASH_ACTIVE: &str = "dash.active";
pub const DASH_INACTIVE: &str = "dash.inactive";
pub const DASH_AGENT_SWITCHED: &str = "dash.agent_switched";
pub const DASH_NO_SWITCH: &str = "dash.no_switch";
pub const DASH_AGENT_ADDED: &str = "dash.agent_added";
pub const DASH_AGENT_ADD_FAIL: &str = "dash.agent_add_fail";

// === CLI/Headless ===
pub const CLI_NO_CONFIG: &str = "cli.no_config";
pub const CLI_ENTER_PW: &str = "cli.enter_pw";
pub const CLI_NO_TELEGRAM: &str = "cli.no_telegram";
pub const CLI_PAIRED: &str = "cli.paired";
pub const CLI_CHAT_SAVED: &str = "cli.chat_saved";
pub const CLI_CHAT_SAVE_FAIL: &str = "cli.chat_save_fail";
pub const CLI_MSG_RECEIVED: &str = "cli.msg_received";
pub const CLI_BOT_SHUTDOWN: &str = "cli.bot_shutdown";
pub const CLI_GRACEFUL: &str = "cli.graceful_shutdown";

// === 도구 하네스 ===
pub const TOOL_LEVEL_SAFE: &str = "tool.level.safe";
pub const TOOL_LEVEL_JAIL: &str = "tool.level.jail";
pub const TOOL_LEVEL_RESTRICTED: &str = "tool.level.restricted";
pub const TOOL_FILE_READ_NAME: &str = "tool.file_read.name";
pub const TOOL_FILE_WRITE_NAME: &str = "tool.file_write.name";
pub const TOOL_FILE_LIST_NAME: &str = "tool.file_list.name";
pub const TOOL_SLEEP_NAME: &str = "tool.sleep.name";
pub const TOOL_PRINT_NAME: &str = "tool.print.name";

// === 텔레그램 봇 ===
pub const BOT_PAIR_PROMPT: &str = "bot.pair_prompt";
pub const BOT_PAIR_SUCCESS: &str = "bot.pair_success";
pub const BOT_PAIR_FAIL: &str = "bot.pair_fail";
pub const BOT_HELP: &str = "bot.help";

// === DB ===
pub const DB_TYPE_USER_MSG: &str = "db.type.user_msg";
pub const DB_TYPE_AGENT_RESP: &str = "db.type.agent_resp";
pub const DB_TYPE_FILE_OP: &str = "db.type.file_op";
pub const DB_TYPE_CONFIG_CHANGE: &str = "db.type.config_change";
pub const DB_TYPE_TOOL_CALL: &str = "db.type.tool_call";
pub const DB_TYPE_SECURITY_EVENT: &str = "db.type.security_event";

// === 검증 ===
pub const VAL_TIMEOUT: &str = "val.timeout";
pub const VAL_CONNECT_FAIL: &str = "val.connect_fail";
pub const VAL_BOT_CONFIRMED: &str = "val.bot_confirmed";
pub const VAL_CHECK_TOKEN: &str = "val.check_token";
