<div align="center">

# CC Switch Mini

### Server-seitiger Abkömmling von [cc-switch](https://github.com/farion1231/cc-switch) · ohne GUI · Verwaltung im Browser

[![Version](https://img.shields.io/github/v/release/yuexiaoliang/cc-switch-mini?color=blue&label=version)](https://github.com/yuexiaoliang/cc-switch-mini/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/yuexiaoliang/cc-switch-mini/releases)
[![Built with Axum](https://img.shields.io/badge/built%20with-Axum%200.7-orange.svg)](https://github.com/tokio-rs/axum)
[![Upstream](https://img.shields.io/badge/upstream-cc--switch-blue)](https://github.com/farion1231/cc-switch)

### 🌐 Die offizielle Webseite: **[ccswitch.io](https://ccswitch.io)**

[English](README.md) | [中文](README_ZH.md) | [日本語](README_JA.md) | Deutsch | [Änderungsprotokoll](CHANGELOG.md)

</div>

> **Desktop-Nutzer: verwendet das Original [cc-switch](https://github.com/farion1231/cc-switch).**
> Dieses Repo ist eine Übergangslösung für headless Server, bis das Upstream-Projekt eine offizielle Server-Version veröffentlicht.

## Highlights

- **Einzelnes Binärpaket** (`cc-switch-mini`), das Web-UI und lokale SQLite-Datenbank bündelt.
- **Null Änderungen am Upstream-Code**: vorhandene Rust-Geschäftslogik (`ProviderService` / `ProxyService` / `ConfigService` / `McpService` / `Database` ...) und React-Frontend werden unverändert wiederverwendet.
- **Tauri IPC wird ersetzt**: ein schlanker HTTP/SSE-Bridge in `.ccsm/bridge/` ersetzt Tauri-IPC. pnpm `overrides` leiten `@tauri-apps/*`-Importe auf das lokale Bridge-Paket um, das Frontend kompiliert ohne Änderungen.
- **Vollständige Feature-Parität**: Jeder `invoke` aus dem Frontend wird in `.ccsm/server/src/commands_extra/*` beantwortet. Siehe den Abschnitt [Feature-Parität mit Upstream](#feature-parität-mit-upstream).
- **Gleicher Quellcode, zwei Binaries**: Der Tauri-Desktop-Build funktioniert weiterhin; `cargo build -p cc-switch-mini-server` erzeugt den Headless-Server.

## Installation

Einzeiler (Linux x64 / arm64, macOS):

```
curl -fsSL https://raw.githubusercontent.com/yuexiaoliang/cc-switch-mini/main/.ccsm/scripts/install.sh | sh
```

Nach der Installation in `/usr/local/bin` starten und per SSH-Tunnel darauf zugreifen:

```
ssh -L 3000:localhost:3000 user@host
open http://localhost:3000
```

Standardmäßig bindet der Server nur an `127.0.0.1:3000` (sicherer Standard). `--host 0.0.0.0` nur verwenden, wenn die Sicherheitsauswirkungen verstanden wurden (Konfiguration enthält API-Keys).

## Aus dem Quellcode bauen

```
pnpm install
pnpm run build:renderer    # schreibt nach dist/
cargo build --release -p cc-switch-mini-server
target/release/cc-switch-mini --help
```

`build:renderer` führt Vite aus. Das Ergebnis wird via `include_dir!` in das Rust-Binary eingebettet, sodass eine komplett eigenständige Binärdatei entsteht.

## Konfiguration

| Flag | Standard | Beschreibung |
| --- | --- | --- |
| `--host` | `127.0.0.1` | Bind-Adresse. `0.0.0.0` legt das UI offen (`--token` mitgeben) |
| `--port` | `3000` | Listen-Port |
| `--data-dir` | `~/.local/share/cc-switch-mini` | Logs & temporäre Dateien (DB lebt unter dem Upstream-Pfad `~/.cc-switch/`) |
| `--config-dir` | Benutzer-Home | Überschreibt die Konfig-Verzeichnisse von Claude / Codex / Gemini |
| `--token` | _(keines)_ | Optionaler Bearer-Token; jede `/api/*`-Anfrage muss ihn mitsenden |
| `--no-spa-fallback` | aus | Unbekannte Pfade direkt 404 (zum Debuggen des Bridges) |

Entsprechende Umgebungsvariablen: `CC_SWITCH_MINI_DATA_DIR`, `CC_SWITCH_MINI_CONFIG_DIR`, `CC_SWITCH_MINI_TOKEN`.

> **Upstream-Kompatibilität** — `--data-dir` ist cc-switch-mini-spezifisch. Die SQLite-Datenbank und die Provider-Konfigurationen (`~/.hermes/`, `~/.claude/`, `~/.codex/`, `~/.gemini/`) liegen immer an den Upstream-Pfaden, sodass ein Wechsel zur Desktop-App ohne Datenmigration möglich ist.

## Feature-Parität mit Upstream

`bash .ccsm/scripts/check-coverage.sh` extrahiert alle `invoke("<cmd>")` aus `src/lib/api/` und gleicht sie mit der Dispatch-Tabelle in `.ccsm/server/src/dispatch.rs` ab. Mit v3.16.2 sind alle Frontend-Befehle abgedeckt (Exit-Code 0). Aufschlüsselung nach Bereichen:

| Bereich | Status | Anmerkungen |
| --- | --- | --- |
| **Provider CRUD** | ✅ Vollständig | `get_providers`, `add_provider`, `update_provider`, `delete_provider`, `switch_provider`, `update_providers_sort_order`, `remove_provider_from_live_config`, `import_default_config`, `sync_current_providers_live`. Nutzt `ProviderService` direkt. |
| **Provider-Hilfsroutinen** | ✅ Vollständig | `get_current_provider`, `read_live_provider_settings`, `get_custom_endpoints`, `add_custom_endpoint`, `remove_custom_endpoint`, `update_endpoint_last_used`, plus Universal-Provider-Familie (`get_universal_providers` / `get_universal_provider` / `upsert_universal_provider` / `delete_universal_provider` / `sync_universal_provider`). Universal-Provider werden direkt in der `settings`-Tabelle gespeichert. |
| **Einstellungen** | ✅ Vollständig | `get_settings`, `save_settings`, plus Snippet-Endpunkte (`get_claude_common_config_snippet`, `set_claude_common_config_snippet`, `get_common_config_snippet`, `set_common_config_snippet`). |
| **Proxy** | ✅ Vollständig | `start_proxy_server`, `stop_proxy_with_restore`, `get_proxy_status`, `is_proxy_running`, `get_proxy_config`, `update_proxy_config`, `get_global_proxy_config`, `update_global_proxy_config`, `set_global_proxy_url`, `get_proxy_config_for_app`, `set_proxy_takeover_for_app`, `get_proxy_takeover_status`, `is_live_takeover_active`. |
| **Failover** | ✅ Vollständig | `get_failover_queue`, `add_to_failover_queue`, `remove_from_failover_queue`, `get_auto_failover_enabled`, `set_auto_failover_enabled`, `get_available_providers_for_failover`. Direkt auf der Provider-Tabelle. |
| **Stream-Check** | ✅ Vollständig | `stream_check_provider`, `stream_check_all_providers`, `get_stream_check_config`, `save_stream_check_config`. Lokaler HTTP-Probe (Anthropic-Stil `/v1/messages`). |
| **MCP-Server** | ✅ Vollständig | Vereinheitlichte API: `get_mcp_servers`, `upsert_mcp_server`, `delete_mcp_server`, `toggle_mcp_app`, `set_mcp_enabled`, `import_mcp_from_apps`, Legacy `upsert_mcp_server_in_config` / `delete_mcp_server_in_config`. Alles über `McpService`. |
| **Claude-MCP** | ✅ Vollständig | `get_claude_mcp_status`, `read_claude_mcp_config`, `upsert_claude_mcp_server`, `delete_claude_mcp_server`, `validate_mcp_command` — re-exportierte Upstream-Befehle lesen/schreiben `~/.claude.json`. |
| **Prompts** | ✅ Vollständig | `get_prompts`, `upsert_prompt`, `delete_prompt`, `enable_prompt`, `import_prompt_from_file`, `get_current_prompt_file_content`. Direktes SQL auf `prompts` (Upstream-`Prompt`-Typ ist privat). |
| **Skills** | ⚠️ Lesen dominant | `get_installed_skills`, `get_skill_backups`, `delete_skill_backup`, `toggle_skill_app`, `scan_unmanaged_skills`, `migrate_skill_storage`, `uninstall_skill_unified`, `uninstall_skill`. Install / Update / Suche sind gestubbt, weil `DiscoverableSkill` / `SkillRepo` / `SkillUpdateInfo` privat sind. Installation via Desktop-App, danach im Web-UI sichtbar. |
| **Nutzungsstatistiken** | ⚠️ Lesen-only | `get_usage_summary`, `get_usage_summary_by_app`, `get_usage_trends`, `get_provider_stats`, `get_model_stats`, `get_request_logs`, `get_request_detail`, `check_provider_limits`, `get_model_pricing`, `update_model_pricing`, `delete_model_pricing`. `sync_session_usage` ist ein Stub (`services::session_usage::*` privat). Echte Synchronisation via Desktop-App. |
| **Sitzungen** | ✅ Vollständig | `list_sessions`, `get_session_messages`, `delete_session`, `delete_sessions` werden per re-export weitergereicht. `launch_session_terminal` ist ein Stub (kein Terminal auf dem Server). |
| **Hermes** | ✅ Vollständig | `get_hermes_live_provider_ids`, `get_openclaw_live_provider`, `import_hermes_providers_from_live`, `get_hermes_model_config`, `open_hermes_web_ui`, `launch_hermes_dashboard`, `get_hermes_memory`, `set_hermes_memory`, `get_hermes_memory_limits`, `set_hermes_memory_enabled`. |
| **OpenClaw-Editor** | ✅ Vollständig | `get_openclaw_live_provider_ids`, `get_openclaw_live_provider`, `import_openclaw_providers_from_live`, `scan_openclaw_config_health`, plus `agents.defaults` / `env` / `tools` / `model` / `models`-Editoren. `openclaw.json` wird direkt gelesen/geschrieben (Upstream-Modul privat). |
| **OpenCode** | ✅ Vollständig | `get_opencode_live_provider_ids`, `import_opencode_providers_from_live`. |
| **OMO** | ⚠️ Lesen-only | `read_omo_local_file`, `read_omo_slim_local_file`, `get_current_omo_provider_id`, `get_current_omo_slim_provider_id`, `disable_current_omo`, `disable_current_omo_slim`. |
| **Deep-Links** | ⚠️ Lesen dominant | `parse_deeplink`, `merge_deeplink_config`, `import_from_deeplink_unified` (nur Provider-Pfad). MCP / Skill-Pfade geben einen Fehler zurück, der auf `import_mcp_from_apps` oder die Desktop-App verweist. |
| **DB-Backup** | ✅ Vollständig | `create_db_backup`, `list_db_backups`, `delete_db_backup`, `restore_db_backup`, `rename_db_backup` — Datei-Kopie. |
| **S3 / WebDAV-Sync** | ⚠️ Nur Einstellungen | `s3_sync_save_settings` / `webdav_sync_save_settings` legen die Konfiguration in der `settings`-Tabelle ab. `*_sync_upload` / `*_sync_download` / `*_test_connection` / `*_sync_fetch_remote_info` sind Stubs (`services::s3_sync`, `services::webdav_sync` privat). Tatsächliche Übertragung via Desktop-App. |
| **Codex OAuth** | ⚠️ Stub | `get_codex_oauth_quota`, `get_codex_oauth_models` liefern `null` (OAuth-Manager hängt an der Tauri-Laufzeit). OAuth via Desktop-App. |
| **GitHub / Copilot-Auth** | ⚠️ Stub | `auth_get_status`, `auth_list_accounts`, `auth_logout`, `auth_remove_account`, `auth_set_default_account`, `copilot_*` und Account-Varianten liefern deterministisch leere Status. OAuth-Flows brauchen die Desktop-App. |
| **Desktop-only-Befehle** | ⚠️ Stub | `open_external` loggt nur die URL. `open_file_dialog` / `save_file_dialog` / `open_zip_file_dialog` / `pick_directory` / `open_app_config_folder` / `open_config_folder` / `open_provider_terminal` / `open_workspace_directory` / `set_window_theme` / `restart_app` / `set_auto_launch` / `is_portable_mode` / `get_auto_launch_status` / `check_for_updates` / `update_tray_menu` sind Desktop-/Fenster-Befehle. |
| **Sitzungs-Log-Sync / Live-Stats** | ⚠️ Stub | `sync_session_usage`, `queryProviderUsage`, `testUsageScript`, `test_api_endpoints` benötigen den laufenden Proxy. |
| **Custom-Endpoint-Tracking** | ⚠️ Stub | `get_custom_endpoints`, `add_custom_endpoint`, `remove_custom_endpoint`, `update_endpoint_last_used` nehmen den Aufruf an und bestätigen. Vollständige Implementierung erfordert Zugriff auf private `Provider`-Felder. |
| **Claude Desktop** | ⚠️ Stub | `get_claude_desktop_status` prüft das Dateisystem. `get_claude_desktop_default_routes`, `import_claude_desktop_providers_from_claude`, `ensure_claude_desktop_official_provider` sind Stubs. |

### Wie funktionieren die Stubs?

Das cc-switch-Frontend behandelt nicht implementierte Befehle als 404 und fällt auf eine "Funktion nicht verfügbar"-Anzeige zurück. cc-switch-mini gibt leere Werte (`[]`, `null`, `false`) oder Logmeldungen zurück, sodass das Frontend nicht abstürzt, sondern die entsprechende Schaltfläche deaktiviert. Dies entspricht dem "Keine Geschäftslogik"-Vertrag: Die Dispatch-Schicht ist ein dünner Durchreicher.

Wenn der Upstream-Typ in einem `pub mod` liegt, binden wir ihn direkt ein; ist er privat, arbeiten wir direkt auf der Datenbank / Datei. Siehe Datei-Kopfkommentare in `.ccsm/server/src/commands_extra/*.rs` für die Begründung pro Befehl.

## Architektur

```
+-------------------+        +-----------------------------+
| Browser           |  HTTP  | cc-switch-mini              |
|                   |  ----> |                             |
|  - React SPA      |        |  Axum-Router                |
|  - bridge/*       |        |    POST /api/invoke/<cmd>   |
|    (ersetzt       |  SSE   |    GET  /api/events         |
|     @tauri-apps/  | <----  |    GET  /api/health         |
|     api/*)        |        |    GET  /<file>  (SPA)      |
|                   |        |                             |
|                   |        |  dispatch  ->  cc_switch_lib (Upstream)
|                   |        |              - ProviderService
|                   |        |              - ProxyService
|                   |        |              - ConfigService
|                   |        |              - McpService
|                   |        |              - StreamCheckService
|                   |        |              - Database (SQLite)
+-------------------+        +-----------------------------+
```

Die Bridge ist eine winzige TypeScript-Schicht: `@tauri-apps/api/core` `invoke` wird zu `POST /api/invoke/<cmd>` und `@tauri-apps/api/event` `listen` wird ein `GET /api/events` SSE-Konsument. Die übrigen `@tauri-apps/*`-Pakete (`window`, `app`, `path`, `plugin-dialog`, `plugin-process`, `plugin-store`, `plugin-updater`) sind Stubs oder dünne Fallbacks.

## Entwicklungs-Workflow

| Aufgabe | Befehl |
| --- | --- |
| Dev-Server starten | `cargo run -p cc-switch-mini-server` |
| Dev-Server mit Port | `cargo run -p cc-switch-mini-server -- --port 8080` |
| Frontend Typ-Check | `pnpm run typecheck` |
| Release bauen | `pnpm run build:renderer && cargo build --release -p cc-switch-mini-server` |
| Dispatch-Abdeckung prüfen | `bash .ccsm/scripts/check-coverage.sh` |
| Unit-Tests ausführen | `cargo test -p cc-switch-mini-server` |
| Upstream synchronisieren (Maintainer) | `bash .ccsm/scripts/sync-upstream.sh` |

## Sync-Strategie

Das Repo ist ein Fork. `.ccsm/scripts/sync-upstream.sh` führt `git fetch && git merge upstream/main` aus und wendet die Konfliktauflösungsrichtlinie an:

1. Übernimmt `src/` und `src-tauri/` unverändert vom Upstream.
2. Behält unsere `.ccsm/`, die Wurzel-`Cargo.toml` und die `pnpm.overrides` in `package.json`.

Die Versionsnummer in `Cargo.toml` / `package.json` spiegelt stets das Upstream-Tag wider; das Installationsskript und der Release-Workflow holen das passende Binary vom Release mit demselben Tag.

## Lizenz

MIT(vom Upstream geerbt).
