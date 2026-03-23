// femtoClaw — 日本語メッセージマップ
// [v0.5.0] Japanese message map

/// 日本語メッセージ検索
pub fn get(key: &str) -> Option<&'static str> {
    Some(match key {
        // === エラー ===
        "err.home_not_found" => "ホームディレクトリが見つかりません",
        "err.sandbox_create" => "サンドボックスディレクトリの作成に失敗: {}",
        "err.already_running" => "femtoClawは既に実行中です (PID: {})",
        "err.lock_file" => "ロックファイルエラー: {}",
        "err.key_derivation" => "暗号鍵の生成に失敗",
        "err.encryption" => "データの暗号化に失敗",
        "err.decryption" => "復号化に失敗: パスワードが正しくないかデータが破損しています",
        "err.config_io" => "設定ファイルI/Oエラー: {}",
        "err.invalid_config" => "設定ファイルの形式が正しくありません",
        "err.serialization" => "シリアライズエラー: {}",
        "err.max_agents" => "エージェントは最大3つまで登録可能です",
        "err.http_client" => "HTTPクライアントの作成に失敗: {}",

        // === パスワード ===
        "pw.empty" => "パスワードを入力してください",
        "pw.too_short" => "4文字以上入力してください",
        "pw.mismatch" => "パスワードが一致しません",
        "pw.key_generated" => "マスターキー生成完了",
        "pw.save_fail" => "設定の保存に失敗: {}",
        "pw.decrypt_ok" => "設定の復号化に成功",
        "pw.3fail_reset" => "3回失敗。[R]で設定をリセットしてください",
        "pw.wrong_pw" => "パスワード誤り ({}/3)",

        // === オンボーディング ===
        "onboard.save_ok" => "設定保存完了 → ダッシュボード",
        "onboard.save_fail" => "❌ 保存失敗: {} — ディスク容量/権限を確認してください",
        "onboard.llm_status_wait" => "検証待ち",
        "onboard.llm_status_testing" => "検証中 (最大5秒)",
        "onboard.llm_status_fail_retry" => "[V]でリトライ",
        "onboard.tg_status_wait" => "検証待ち (任意)",
        "onboard.tg_status_testing" => "検証中 (最大5秒)",
        "onboard.tg_status_ok" => "Telegram Bot 確認済み",
        "onboard.tg_status_fail_retry" => "[V]でリトライ",

        // === ブート ===
        "boot.init_msg" => "femtoClaw 起動中",

        // === フィード ===
        "feed.llm_verify_ok" => "LLM検証成功: {} — {}個のモデル検出",
        "feed.llm_verify_ok_simple" => "LLM検証成功: {}",
        "feed.llm_verify_fail" => "LLM検証失敗: {}",
        "feed.tg_verify_ok" => "Telegram検証成功: {}",
        "feed.tg_verify_fail" => "Telegram検証失敗: {}",

        // === ダッシュボード ===
        "dash.agent_status" => "━━ エージェント状態 ━━",
        "dash.agent_name" => "エージェント: {}",
        "dash.model" => "モデル: {} ({:?})",
        "dash.security" => "セキュリティ: Jailing=ON | ChaCha20=ON",
        "dash.llm_none" => "LLM未設定",
        "dash.skill_header" => "━━ スキル一覧 (TOML + Rhai) ━━",
        "dash.skill_builtin" => "[内蔵][{}] {} — {}",
        "dash.skill_core_fail" => "コア読み込み失敗: {}",
        "dash.skill_user" => "[ユーザー][{}] {} — {}",
        "dash.skill_user_fail" => "ユーザー読み込み失敗: {}",
        "dash.timemachine_header" => "━━ タイムマシン (直近10件) ━━",
        "dash.timemachine_cols" => "種別|状態|概要|時刻",
        "dash.no_records" => "(記録なし)",
        "dash.total_count" => "── 全 {} 件 ──",
        "dash.db_query_fail" => "DBクエリ失敗: {}",
        "dash.db_open_fail" => "DBオープン失敗: {}",
        "dash.agent_switch_header" => "━━ エージェント切替 ━━",
        "dash.no_agents" => "(登録済みエージェントなし)",
        "dash.active" => "有効",
        "dash.inactive" => "無効",
        "dash.agent_switched" => "→ エージェント#{}({})に切替!",
        "dash.no_switch" => "(切替可能な他のエージェントなし)",
        "dash.agent_added" => "✅ エージェント#{} ({}) 追加完了",
        "dash.agent_add_fail" => "❌ {}",

        // === CLI ===
        "cli.no_config" => "config.encがありません。TUIモードで先に設定してください",
        "cli.enter_pw" => "マスターパスワード: ",
        "cli.no_telegram" => "Telegram未設定または未検証です",
        "cli.paired" => "ペアリング成功: {} (chat_id: {})",
        "cli.chat_saved" => "chat_id保存完了 — 再起動後もペアリング維持",
        "cli.chat_save_fail" => "ペアリングchat_id保存失敗: {}",
        "cli.msg_received" => "メッセージ受信: {}",
        "cli.bot_shutdown" => "ボット終了",
        "cli.graceful_shutdown" => "Graceful Shutdown完了",

        // === ツール ===
        "tool.level.safe" => "安全",
        "tool.level.jail" => "Jail検証",
        "tool.level.restricted" => "制限",
        "tool.file_read.name" => "ファイル読取",
        "tool.file_write.name" => "ファイル書込",
        "tool.file_list.name" => "ディレクトリ一覧",
        "tool.sleep.name" => "待機",
        "tool.print.name" => "ログ出力",

        // === Telegramボット ===
        "bot.pair_prompt" => "🔐 ペアリングが必要です。/pair <PIN>で認証してください。",
        "bot.pair_success" => "✅ ペアリング完了！会話が可能です。",
        "bot.pair_fail" => "❌ PINが一致しません。再度お試しください。",
        "bot.help" => "📋 コマンド:\n/pair <PIN> — ペアリング\n/status — 状態\n/agent <N> — 切替",

        // === DB ===
        "db.type.user_msg" => "ユーザーメッセージ",
        "db.type.agent_resp" => "エージェント応答",
        "db.type.file_op" => "ファイル操作",
        "db.type.config_change" => "設定変更",
        "db.type.tool_call" => "ツール呼出",
        "db.type.security_event" => "セキュリティイベント",

        // === 検証 ===
        "val.timeout" => "タイムアウト (5秒)",
        "val.connect_fail" => "接続失敗 — サーバーが稼働中か確認してください",
        "val.bot_confirmed" => "ボット確認済み",
        "val.check_token" => "トークンが正しいか確認してください",

        _ => return None,
    })
}
