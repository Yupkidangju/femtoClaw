// femtoClaw — English message map
// [v0.5.0] Default fallback language for unsupported OS locales.

/// English message lookup
pub fn get(key: &str) -> Option<&'static str> {
    Some(match key {
        // === Errors ===
        "err.home_not_found" => "Home directory not found",
        "err.sandbox_create" => "Failed to create sandbox directory: {}",
        "err.already_running" => "femtoClaw is already running (PID: {})",
        "err.lock_file" => "Lock file error: {}",
        "err.key_derivation" => "Encryption key derivation failed",
        "err.encryption" => "Data encryption failed",
        "err.decryption" => "Decryption failed: incorrect password or corrupted data",
        "err.config_io" => "Config file I/O error: {}",
        "err.invalid_config" => "Invalid config file format",
        "err.serialization" => "Serialization error: {}",
        "err.max_agents" => "Maximum of 3 agents allowed",
        "err.http_client" => "Failed to create HTTP client: {}",

        // === Password ===
        "pw.empty" => "Please enter a password",
        "pw.too_short" => "Minimum 4 characters required",
        "pw.mismatch" => "Passwords do not match",
        "pw.key_generated" => "Master key generated",
        "pw.save_fail" => "Failed to save config: {}",
        "pw.decrypt_ok" => "Config decrypted successfully",
        "pw.3fail_reset" => "3 failed attempts. Press [R] to reset",
        "pw.wrong_pw" => "Wrong password ({}/3)",

        // === Onboarding ===
        "onboard.save_ok" => "Config saved → Dashboard",
        "onboard.save_fail" => "❌ Failed to save: {} — Check disk space/permissions",
        "onboard.llm_status_wait" => "Awaiting verification",
        "onboard.llm_status_testing" => "Verifying (max 5s)",
        "onboard.llm_status_fail_retry" => "Press [V] to retry",
        "onboard.tg_status_wait" => "Awaiting verification (optional)",
        "onboard.tg_status_testing" => "Verifying (max 5s)",
        "onboard.tg_status_ok" => "Telegram Bot confirmed",
        "onboard.tg_status_fail_retry" => "Press [V] to retry",

        // === Boot ===
        "boot.init_msg" => "femtoClaw starting",

        // === Feed ===
        "feed.llm_verify_ok" => "LLM verified: {} — {} models found",
        "feed.llm_verify_ok_simple" => "LLM verified: {}",
        "feed.llm_verify_fail" => "LLM verification failed: {}",
        "feed.tg_verify_ok" => "Telegram verified: {}",
        "feed.tg_verify_fail" => "Telegram verification failed: {}",

        // === Dashboard ===
        "dash.agent_status" => "━━ Agent Status ━━",
        "dash.agent_name" => "Agent: {}",
        "dash.model" => "Model: {} ({})",
        "dash.security" => "Security: Jailing=ON | ChaCha20=ON",
        "dash.llm_none" => "LLM not configured",
        "dash.skill_header" => "━━ Skills (TOML + Rhai) ━━",
        "dash.skill_builtin" => "[Built-in][{}] {} — {}",
        "dash.skill_core_fail" => "Core load failed: {}",
        "dash.skill_user" => "[User][{}] {} — {}",
        "dash.skill_user_fail" => "User load failed: {}",
        "dash.timemachine_header" => "━━ Time Machine (Last 10) ━━",
        "dash.timemachine_cols" => "Type|Status|Summary|Time",
        "dash.no_records" => "(No records)",
        "dash.total_count" => "── Total {} records ──",
        "dash.db_query_fail" => "DB query failed: {}",
        "dash.db_open_fail" => "DB open failed: {}",
        "dash.agent_switch_header" => "━━ Agent Switch ━━",
        "dash.no_agents" => "(No agents registered)",
        "dash.active" => "Active",
        "dash.inactive" => "Inactive",
        "dash.agent_switched" => "→ Switched to Agent #{}({})!",
        "dash.no_switch" => "(No other agents to switch to)",
        "dash.agent_added" => "✅ Agent #{} ({}) added",
        "dash.agent_add_fail" => "❌ {}",

        // === CLI/Headless ===
        "cli.no_config" => "config.enc not found. Run TUI mode to set up first",
        "cli.enter_pw" => "Master password: ",
        "cli.no_telegram" => "Telegram not configured or unverified",
        "cli.paired" => "Paired: {} (chat_id: {})",
        "cli.chat_saved" => "chat_id saved — pairing persists across restarts",
        "cli.chat_save_fail" => "Failed to save pairing chat_id: {}",
        "cli.msg_received" => "Message received: {}",
        "cli.bot_shutdown" => "Bot shut down",
        "cli.graceful_shutdown" => "Graceful shutdown complete",

        // === Tools ===
        "tool.level.safe" => "Safe",
        "tool.level.jail" => "Jail Required",
        "tool.level.restricted" => "Restricted",
        "tool.file_read.name" => "File Read",
        "tool.file_write.name" => "File Write",
        "tool.file_list.name" => "Directory Listing",
        "tool.sleep.name" => "Sleep",
        "tool.print.name" => "Print Log",

        // === Telegram Bot ===
        "bot.pair_prompt" => "🔐 Pairing required. Use /pair <PIN> to authenticate.",
        "bot.pair_success" => "✅ Paired! You can now chat.",
        "bot.pair_fail" => "❌ Incorrect PIN. Please try again.",
        "bot.help" => "📋 Commands:\n/pair <PIN> — Pair\n/status — Status\n/agent <N> — Switch",

        // === DB ===
        "db.type.user_msg" => "Chat",
        "db.type.agent_resp" => "Response",
        "db.type.file_op" => "File",
        "db.type.api_call" => "API",
        "db.type.system_event" => "System",
        "db.type.skill_run" => "Skill",
        "db.type.config_change" => "Config Change",
        "db.type.tool_call" => "Tool Call",
        "db.type.security_event" => "Security Event",

        // === Validation ===
        "val.timeout" => "Timeout (5s)",
        "val.connect_fail" => "Connection failed — check if server is running",
        "val.bot_confirmed" => "Bot confirmed",
        "val.check_token" => "Check if token is correct",

        _ => return None,
    })
}
