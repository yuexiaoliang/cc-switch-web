<div align="center">

# CC Switch Mini

### [cc-switch](https://github.com/farion1231/cc-switch) のサーバーサイド派生版 · GUI 不要 · ブラウザーで管理

[![Version](https://img.shields.io/github/v/release/yuexiaoliang/cc-switch-web?color=blue&label=version)](https://github.com/yuexiaoliang/cc-switch-web/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/yuexiaoliang/cc-switch-web/releases)
[![Built with Axum](https://img.shields.io/badge/built%20with-Axum%200.7-orange.svg)](https://github.com/tokio-rs/axum)
[![Upstream](https://img.shields.io/badge/upstream-cc--switch-blue)](https://github.com/farion1231/cc-switch)

### 🌐 公式サイト: **[ccswitch.io](https://ccswitch.io)**

[English](README.md) | [中文](README_ZH.md) | 日本語 | [Deutsch](README_DE.md) | [更新履歴](CHANGELOG.md)

</div>

> **デスクトップユーザー: オリジナルの [cc-switch](https://github.com/farion1231/cc-switch) を使用してください。**
> 本プロジェクトは GUI を持たないサーバー向けの派生版で、上流公式サーバー版が出るまでのブリッジです。

## ハイライト

- **単一バイナリ** (`cc-switch-web`)、Web UI とローカル SQLite データベースを同梱。
- **上流コードにゼロ変更**: 既存の Rust ビジネスロジック (`ProviderService` / `ProxyService` / `ConfigService` / `McpService` / `Database` ...) と React フロントエンドをそのまま流用。
- **Tauri IPC を置換**: `.ccsm/bridge/` の薄い HTTP/SSE ブリッジが Tauri IPC を代行。pnpm `overrides` で `@tauri-apps/*` をローカルパッケージにリダイレクトするため、フロントエンドは無修正でビルド可能。
- **完全機能互換**: フロントエンドが発するすべての `invoke` を `.ccsm/server/src/commands_extra/*` でハンドリング。詳細 は [上流との機能対応表](#上流との機能対応表) を参照。
- **同じソースから 2 種類のバイナリ**: 開発者のデスクトップでは上流の Tauri ビルドが動作し、`cargo build -p cc-switch-web-server` でヘッドレスサーバーバイナリを生成。

## インストール

ワンライナー (Linux x64 / arm64、macOS):

```
curl -fsSL https://raw.githubusercontent.com/yuexiaoliang/cc-switch-web/main/.ccsm/scripts/install.sh | sh
```

`/usr/local/bin` にインストールされたら、起動して SSH トンネル経由でアクセスします。

```
ssh -L 3000:localhost:3000 user@host
open http://localhost:3000
```

デフォルトでは `127.0.0.1:3000` のみをリッスンします(安全なデフォルト)。意図的に公開する場合のみ `--host 0.0.0.0` を使用してください(設定に API キーが含まれます)。

## ソースからビルド

```
pnpm install
pnpm run build:renderer    # dist/ に書き出し
cargo build --release -p cc-switch-web-server
target/release/cc-switch-web --help
```

`build:renderer` は Vite を実行し、その出力は `include_dir!` で Rust バイナリに埋め込まれるため、自己完結した単一バイナリが完成します。

## 設定

| フラグ | デフォルト | 説明 |
| --- | --- | --- |
| `--host` | `127.0.0.1` | バインドアドレス。`0.0.0.0` で外部公開(`--token` と併用) |
| `--port` | `3000` | リッスンポート |
| `--data-dir` | `~/.local/share/cc-switch-web` | ログ・一時ファイル(DB は上流 `~/.cc-switch/`) |
| `--config-dir` | ユーザーのホーム | Claude / Codex / Gemini などの設定ディレクトリ上書き |
| `--token` | _(なし)_ | 任意の Bearer トークン。すべての `/api/*` リクエストで必要 |
| `--no-spa-fallback` | off | 未知のパスを 404 で返す(ブリッジのデバッグ用) |

対応する環境変数: `CC_SWITCH_MINI_DATA_DIR`、`CC_SWITCH_MINI_CONFIG_DIR`、`CC_SWITCH_MINI_TOKEN`。

> **上流互換** — `--data-dir` は cc-switch-web のみの概念です。SQLite データベースとプロバイダ設定ファイル(`~/.hermes/`、`~/.claude/`、`~/.codex/`、`~/.gemini/`)は常に上流と同じパスに置かれるため、cc-switch-web とデスクトップ版を自由に切り替えられます。

## 上流との機能対応表

`bash .ccsm/scripts/check-coverage.sh` は `src/lib/api/` 内のすべての `invoke("<cmd>")` を抽出し、`.ccsm/server/src/dispatch.rs` のディスパッチテーブルと突き合わせます。v3.16.2 時点で全フロントエンドコマンドをカバー済み(終了コード 0)。エリア別の内訳:

| エリア | 状態 | 備考 |
| --- | --- | --- |
| **Provider CRUD** | ✅ 完全対応 | `get_providers`、`add_provider`、`update_provider`、`delete_provider`、`switch_provider`、`update_providers_sort_order`、`remove_provider_from_live_config`、`import_default_config`、`sync_current_providers_live`。`ProviderService` を直接呼び出し。 |
| **Provider 補助 API** | ✅ 完全対応 | `get_current_provider`、`read_live_provider_settings`、`get_custom_endpoints`、`add_custom_endpoint`、`remove_custom_endpoint`、`update_endpoint_last_used`、および universal provider 関連 (`get_universal_providers` / `get_universal_provider` / `upsert_universal_provider` / `delete_universal_provider` / `sync_universal_provider`)。Universal provider は settings テーブルへ直接 SQL で読み書き。 |
| **設定 (Settings)** | ✅ 完全対応 | `get_settings`、`save_settings`、共通設定スニペット (`get_claude_common_config_snippet`、`set_claude_common_config_snippet`、`get_common_config_snippet`、`set_common_config_snippet`)。 |
| **プロキシ (Proxy)** | ✅ 完全対応 | `start_proxy_server`、`stop_proxy_with_restore`、`get_proxy_status`、`is_proxy_running`、`get_proxy_config`、`update_proxy_config`、`get_global_proxy_config`、`update_global_proxy_config`、`set_global_proxy_url`、`get_proxy_config_for_app`、`set_proxy_takeover_for_app`、`get_proxy_takeover_status`、`is_live_takeover_active`。 |
| **フェイルオーバー** | ✅ 完全対応 | `get_failover_queue`、`add_to_failover_queue`、`remove_from_failover_queue`、`get_auto_failover_enabled`、`set_auto_failover_enabled`、`get_available_providers_for_failover`。providers テーブルを直接操作。 |
| **ストリームチェック** | ✅ 完全対応 | `stream_check_provider`、`stream_check_all_providers`、`get_stream_check_config`、`save_stream_check_config`。ローカル HTTP プローブ (Anthropic 互換 `/v1/messages`)。 |
| **MCP サーバー** | ✅ 完全対応 | 統一 API `get_mcp_servers`、`upsert_mcp_server`、`delete_mcp_server`、`toggle_mcp_app`、`set_mcp_enabled`、`import_mcp_from_apps`、レガシー `upsert_mcp_server_in_config` / `delete_mcp_server_in_config`。すべて `McpService` 経由。 |
| **Claude MCP** | ✅ 完全対応 | `get_claude_mcp_status`、`read_claude_mcp_config`、`upsert_claude_mcp_server`、`delete_claude_mcp_server`、`validate_mcp_command` —— re-export された上流コマンドで `~/.claude.json` を読み書き。 |
| **プロンプト** | ✅ 完全対応 | `get_prompts`、`upsert_prompt`、`delete_prompt`、`enable_prompt`、`import_prompt_from_file`、`get_current_prompt_file_content`。`prompts` テーブルへ直接 SQL(上流 `Prompt` 型が private)。 |
| **スキル (Skills)** | ⚠️ 読み取り中心 | `get_installed_skills`、`get_skill_backups`、`delete_skill_backup`、`toggle_skill_app`、`scan_unmanaged_skills`、`migrate_skill_storage`、`uninstall_skill_unified`、`uninstall_skill`。インストール / アップデート / 検索はスタブ(`DiscoverableSkill`、`SkillRepo`、`SkillUpdateInfo` 型が private)。デスクトップ版でインストールし、Web UI で結果を確認することを推奨。 |
| **使用統計** | ⚠️ 読み取り専用 | `get_usage_summary`、`get_usage_summary_by_app`、`get_usage_trends`、`get_provider_stats`、`get_model_stats`、`get_request_logs`、`get_request_detail`、`check_provider_limits`、`get_model_pricing`、`update_model_pricing`、`delete_model_pricing`。`sync_session_usage` はスタブ(`services::session_usage::*` が private)。実同期はデスクトップ版で実施。 |
| **セッション** | ✅ 完全対応 | `list_sessions`、`get_session_messages`、`delete_session`、`delete_sessions` を re-export で転送。`launch_session_terminal` はスタブ(サーバー側にターミナルがないため、ユーザーが AI CLI を手動で起動する想定)。 |
| **Hermes** | ✅ 完全対応 | `get_hermes_live_provider_ids`、`get_hermes_live_provider`、`import_hermes_providers_from_live`、`get_hermes_model_config`、`open_hermes_web_ui`、`launch_hermes_dashboard`、`get_hermes_memory`、`set_hermes_memory`、`get_hermes_memory_limits`、`set_hermes_memory_enabled`。 |
| **OpenClaw ライブ編集** | ✅ 完全対応 | `get_openclaw_live_provider_ids`、`get_openclaw_live_provider`、`import_openclaw_providers_from_live`、`scan_openclaw_config_health`、および `agents.defaults` / `env` / `tools` / `model` / `models` エディタ。`openclaw.json` を直接読み書き(上流モジュールが private)。 |
| **OpenCode** | ✅ 完全対応 | `get_opencode_live_provider_ids`、`import_opencode_providers_from_live`。 |
| **OMO** | ⚠️ 読み取り専用 | `read_omo_local_file`、`read_omo_slim_local_file`、`get_current_omo_provider_id`、`get_current_omo_slim_provider_id`、`disable_current_omo`、`disable_current_omo_slim`。 |
| **ディープリンク** | ⚠️ 読み取り中心 | `parse_deeplink`、`merge_deeplink_config`、`import_from_deeplink_unified`(provider 経路のみ)。MCP / Skill 経路はエラー応答を返し、ユーザーに `import_mcp_from_apps` またはデスクトップ版を案内。 |
| **DB バックアップ** | ✅ 完全対応 | `create_db_backup`、`list_db_backups`、`delete_db_backup`、`restore_db_backup`、`rename_db_backup` —— ファイルコピーで実装。 |
| **S3 / WebDAV 同期** | ⚠️ 設定のみ | `s3_sync_save_settings` / `webdav_sync_save_settings` が設定を settings テーブルに永続化。`*_sync_upload` / `*_sync_download` / `*_test_connection` / `*_sync_fetch_remote_info` はスタブ(`services::s3_sync`、`services::webdav_sync` が private)。実転送はデスクトップ版で。 |
| **Codex OAuth** | ⚠️ スタブ | `get_codex_oauth_quota`、`get_codex_oauth_models` は `null` を返す(OAuth マネージャーが Tauri ランタイムに紐付くため)。デスクトップ版で認証を実施。 |
| **GitHub / Copilot 認証** | ⚠️ スタブ | `auth_get_status`、`auth_list_accounts`、`auth_logout`、`auth_remove_account`、`auth_set_default_account`、`copilot_*` はすべて決定論的空ステータスを返す。OAuth フローはデスクトップ版が必要。 |
| **デスクトップ専用コマンド** | ⚠️ スタブ | `open_external` は URL をログするだけ(ブラウザーが無い); `open_file_dialog` / `save_file_dialog` / `open_zip_file_dialog` / `pick_directory` / `open_app_config_folder` / `open_config_folder` / `open_provider_terminal` / `open_workspace_directory` / `set_window_theme` / `restart_app` / `set_auto_launch` / `is_portable_mode` / `get_auto_launch_status` / `check_for_updates` / `update_tray_menu` はデスクトップ専用。 |
| **セッションログ同期 / 統計** | ⚠️ スタブ | `sync_session_usage`、`queryProviderUsage`、`testUsageScript`、`test_api_endpoints` は稼働中のプロキシが必要で、本プログラムは提供しない。 |
| **カスタムエンドポイント** | ⚠️ スタブ | `get_custom_endpoints`、`add_custom_endpoint`、`remove_custom_endpoint`、`update_endpoint_last_used` は呼び出しを受け付けて成功を返すのみ。`Provider` 構造体のフィールドが private なため、完全な実装は上流の変更待ち。 |
| **Claude Desktop** | ⚠️ スタブ | `get_claude_desktop_status` はファイルシステムをプローブ。`get_claude_desktop_default_routes`、`import_claude_desktop_providers_from_claude`、`ensure_claude_desktop_official_provider` はスタブ。 |

### スタブ実装の動作

cc-switch のフロントエンドは未実装コマンドを 404 として扱い、“その機能は無効” という表示にフォールバックします。cc-switch-web は `[]`、`null`、`false` などの空値またはログメッセージを返すため、フロントエンドがクラッシュせず、該当ボタンが無効化されます。これは “ビジネスロジックを書かない” 契約に沿った振る舞いです。

上流の API が型を `pub mod` 配下に置いていれば直接バインドし、private モジュールならデータベース / ファイルへ直接アクセスします。詳細は `.ccsm/server/src/commands_extra/*.rs` 冒頭のコメントを参照してください。

## 日常運用

サーバーが起動したら、インストール時に選択した URL で Web UI にアクセスできます。日々の運用は CLI ラッパーと Web UI の 2 つのインターフェースで行います。

### サービス管理

`~/.local/bin/cc-switch-web-ctl` に便利ラッパーを配置しています:

| サブコマンド | 動作 |
| --- | --- |
| `cc-switch-web-ctl start` | user-systemd サービスを有効化して起動 |
| `cc-switch-web-ctl stop` | サービスを停止 |
| `cc-switch-web-ctl restart` | グレースフル再起動 (`update` 後に自動実行) |
| `cc-switch-web-ctl status` | サービス状態・リスナー・公開エンドポイント・FRP トンネルを一括表示 |
| `cc-switch-web-ctl logs` | `journalctl --user -u cc-switch-web -f` |
| `cc-switch-web-ctl update` | `git pull` + `pnpm install` + `cargo build --release` + 再インストール + 再起動 |

`status` の一回の出力例:

```
=== cc-switch-web service ===
   Active: active (running) since …

=== local listener ===
LISTEN 0  128  127.0.0.1:3000  0.0.0.0:*  users:(("cc-switch-web",pid=…))

=== public endpoint ===
{"status":"ok","version":"3.16.2",…}

=== FRP tunnel ===
   Active: active (running) since …
```

### Web UI の操作

ブラウザで `https://<あなたのドメイン>/` を開きます。Basic 認証を有効にしている場合は (`.ccsm/deploy/README.md` の **設定** セクション参照)、最初に認証プロンプトが表示されます。UI 本体は上流の React アプリで、無改変で配信されています。

| UI 操作 | バックエンド効果 |
| --- | --- |
| アプリタブの切替 (Claude Code / Codex / Hermes / …) | `/api/invoke/get_providers` でそのアプリの provider リストを取得し直し |
| プロバイダ追加 (➕ ボタン) | プリセット一覧を開き、選択したものを `Provider` オブジェクトと共に `/api/invoke/add_provider` に POST |
| プロバイダを有効化 (启用) | `/api/invoke/switch_provider` を POST → 上流の `ProviderService::switch` が `~/.claude/settings.json` (など) を書き換え、`provider-switched` SSE イベントを発火 |
| 編集 / 削除 / URL コピー | 標準的な `/api/invoke/update_provider` / `delete_provider` / `read_live_provider_settings` の往復 |
| 歯車アイコン(設定) | 6 つのタブ(General / Routing / Auth / Advanced / Usage / About)それぞれが対応する `get_*` / `save_*` コマンド群を呼び出す |
| MCP / プロンプト / スキル | 同じパターン — タブごとに薄い `invoke` ラッパー |

完全なディスパッチ対応表(どの UI コマンドがどのサービスメソッドに対応するか、ヘッドレスサーバーでスタブ化されているか) は下部の **上流との機能対応表** セクションを参照してください。

### Web UI を SSH トンネル経由で使う(ドメインを晒さない場合)

パブリックに晒さない場合は SSH トンネルを推奨:

```bash
ssh -L 3000:localhost:3000 user@host
# その後ブラウザで http://localhost:3000
```

サービスはデフォルトで `127.0.0.1:3000` にバインドするため、開発機ではこのトンネル経由が推奨です。

### データの保存場所

cc-switch-web は上流のディレクトリレイアウトをそのまま継承します。デスクトップ版と本サービスの間でインポート/エクスポートなしに切替えられます:

| 内容 | パス | 所有者 |
| --- | --- | --- |
| SQLite データベース | `~/.cc-switch/cc-switch.db` | 両方(上流のスキーマ) |
| Hermes 設定 | `~/.hermes/config.yaml` | 両方 |
| Claude 設定 | `~/.claude/settings.json` | 両方 |
| Codex 設定 | `~/.codex/config.toml` | 両方 |
| Gemini 設定 | `~/.gemini/config.json` | 両方 |
| ログ / 一時ファイル | `~/.local/share/cc-switch-web/` | cc-switch-web のみ |

## アーキテクチャ

```
+-------------------+        +-----------------------------+
| ブラウザー        |  HTTP  | cc-switch-web              |
|                   |  ----> |                             |
|  - React SPA      |        |  Axum ルーター              |
|  - bridge/*       |        |    POST /api/invoke/<cmd>   |
|    (@tauri-apps/  |  SSE   |    GET  /api/events         |
|     api/* を置換)  | <----  |    GET  /api/health         |
|                   |        |    GET  /<file>  (SPA)      |
|                   |        |                             |
|                   |        |  dispatch  ->  cc_switch_lib (上流)
|                   |        |              - ProviderService
|                   |        |              - ProxyService
|                   |        |              - ConfigService
|                   |        |              - McpService
|                   |        |              - StreamCheckService
|                   |        |              - Database (SQLite)
+-------------------+        +-----------------------------+
```

ブリッジは小さな TypeScript レイヤーです。`@tauri-apps/api/core` の `invoke` は `POST /api/invoke/<cmd>` に、`@tauri-apps/api/event` の `listen` は `GET /api/events` SSE コンシューマになります。残りの `@tauri-apps/*` パッケージ(`window`、`app`、`path`、`plugin-dialog`、`plugin-process`、`plugin-store`、`plugin-updater`)はスタブまたは軽量フォールバックです。

## 開発ワークフロー

| タスク | コマンド |
| --- | --- |
| 開発サーバーを起動 | `cargo run -p cc-switch-web-server` |
| ポート指定で起動 | `cargo run -p cc-switch-web-server -- --port 8080` |
| フロントエンド型チェック | `pnpm run typecheck` |
| リリースビルド | `pnpm run build:renderer && cargo build --release -p cc-switch-web-server` |
| dispatch カバレッジ検証 | `bash .ccsm/scripts/check-coverage.sh` |
| ユニットテスト実行 | `cargo test -p cc-switch-web-server` |
| 上流を同期(メンテナー) | `bash .ccsm/scripts/sync-upstream.sh` |

## 同期戦略

本リポジトリは上流のフォークです。`.ccsm/scripts/sync-upstream.sh` が `git fetch && git merge upstream/main` を実行し、以下の競合解決ポリシーを適用します:

1. `src/` と `src-tauri/` は上流版をそのまま採用。
2. `.ccsm/`、ルート `Cargo.toml`、`package.json` の `pnpm.overrides` は本プロジェクト版を保持。

`Cargo.toml` / `package.json` のバージョン番号は常に上流のタグと一致し、インストールスクリプトとリリースフローは同じタグのバイナリを取得します。

## ライセンス

MIT(上流より継承)。
