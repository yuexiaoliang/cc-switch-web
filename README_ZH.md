<div align="center">

# CC Switch Mini

### [cc-switch](https://github.com/farion1231/cc-switch) 的服务端衍生版 · 无桌面环境运行 · 浏览器即可管理

[![Version](https://img.shields.io/github/v/release/yuexiaoliang/cc-switch-web?color=blue&label=version)](https://github.com/yuexiaoliang/cc-switch-web/releases)
[![Platform](https://img.shields.io/badge/platform-Windows%20%7C%20macOS%20%7C%20Linux-lightgrey.svg)](https://github.com/yuexiaoliang/cc-switch-web/releases)
[![Built with Axum](https://img.shields.io/badge/built%20with-Axum%200.7-orange.svg)](https://github.com/tokio-rs/axum)
[![Upstream](https://img.shields.io/badge/upstream-cc--switch-blue)](https://github.com/farion1231/cc-switch)

### 🌐 官方网站：**[ccswitch.io](https://ccswitch.io)**

[English](README.md) | 中文 | [日本語](README_JA.md) | [Deutsch](README_DE.md) | [更新日志](CHANGELOG.md)

</div>

> **桌面用户请使用原版 [cc-switch](https://github.com/farion1231/cc-switch)。**
> 本项目是面向无 GUI 服务器的衍生版本，定位是上游官方服务端版本发布前的过渡方案。

## 项目亮点

- **单二进制** (`cc-switch-web`)，Web UI + 本地 SQLite 数据库。
- **零修改上游**：复用上游 Rust 业务逻辑 (`ProviderService` / `ProxyService` / `ConfigService` / `McpService` / `Database` ...) 和 React 前端代码,逐字未改。
- **替换 Tauri IPC**：用轻量 HTTP/SSE 桥接层替代 `.ccsm/bridge/`,通过 pnpm `overrides` 将 `@tauri-apps/*` 重定向到本地桥接包,前端编译无须改动。
- **完整功能对齐**：上游前端发出的每一条 `invoke` 都能在 `.ccsm/server/src/commands_extra/*` 中找到对应处理器,详见下方 [与上游功能的对齐情况](#与上游功能的对齐情况)。
- **同一份代码，两种构建**：上游 Tauri 桌面版继续可用,`cargo build -p cc-switch-web-server` 即可生成无头服务端二进制。

## 安装

一行命令 (Linux x64 / arm64, macOS)：

```
curl -fsSL https://raw.githubusercontent.com/yuexiaoliang/cc-switch-web/main/.ccsm/scripts/install.sh | sh
```

安装到 `/usr/local/bin` 后,启动并从本地隧道访问:

```
ssh -L 3000:localhost:3000 user@host
open http://localhost:3000         # 或任意浏览器
```

默认只监听 `127.0.0.1:3000`,安全默认值。仅在确认安全的前提下使用 `--host 0.0.0.0` 暴露端口(配置中含有 API Key)。

## 从源码构建

```
pnpm install
pnpm run build:renderer    # 写入 dist/
cargo build --release -p cc-switch-web-server
target/release/cc-switch-web --help
```

`build:renderer` 调用 Vite,产物通过 `include_dir!` 嵌入到 Rust 二进制内,最终得到完全自包含的可执行文件。

## 配置项

| 参数 | 默认值 | 说明 |
| --- | --- | --- |
| `--host` | `127.0.0.1` | 监听地址。`0.0.0.0` 表示对外开放,需配合 `--token` |
| `--port` | `3000` | 监听端口 |
| `--data-dir` | `~/.local/share/cc-switch-web` | 日志与临时文件(数据库位于上游 `~/.cc-switch/`) |
| `--config-dir` | 用户主目录 | 覆盖 Claude / Codex / Gemini 等宿主工具的配置目录 |
| `--token` | _(无)_ | 可选 bearer token,所有 `/api/*` 请求必须携带 |
| `--no-spa-fallback` | 关闭 | 未知路径直接 404(便于调试桥接层) |

对应的环境变量:`CC_SWITCH_MINI_DATA_DIR`、`CC_SWITCH_MINI_CONFIG_DIR`、`CC_SWITCH_MINI_TOKEN`。

> **与上游一致** —— `--data-dir` 是 cc-switch-web 独有的参数。SQLite 数据库与各 provider 配置文件 (`~/.hermes/`、`~/.claude/`、`~/.codex/`、`~/.gemini/`) 始终使用上游路径,可在两套程序之间无缝切换,无须迁移数据。

## 与上游功能的对齐情况

执行 `bash .ccsm/scripts/check-coverage.sh` 会扫描 `src/lib/api/` 中所有的 `invoke("<cmd>")` 调用,并与 `.ccsm/server/src/dispatch.rs` 中的 dispatch 表对比。截至 v3.16.2,前端发出的全部命令均已覆盖(脚本返回 0)。按区域细分:

| 区域 | 状态 | 说明 |
| --- | --- | --- |
| **Provider 增删改查** | ✅ 完全支持 | `get_providers`、`add_provider`、`update_provider`、`delete_provider`、`switch_provider`、`update_providers_sort_order`、`remove_provider_from_live_config`、`import_default_config`、`sync_current_providers_live`。直接调用 `ProviderService`。 |
| **Provider 辅助接口** | ✅ 完全支持 | `get_current_provider`、`read_live_provider_settings`、`get_custom_endpoints`、`add_custom_endpoint`、`remove_custom_endpoint`、`update_endpoint_last_used`,以及 universal provider 家族的 `get_universal_providers` / `get_universal_provider` / `upsert_universal_provider` / `delete_universal_provider` / `sync_universal_provider`。Universal provider 通过 settings 表直接读写。 |
| **设置 (Settings)** | ✅ 完全支持 | `get_settings`、`save_settings`,以及通用配置片段 `get_claude_common_config_snippet`、`set_claude_common_config_snippet`、`get_common_config_snippet`、`set_common_config_snippet`。 |
| **代理 (Proxy)** | ✅ 完全支持 | `start_proxy_server`、`stop_proxy_with_restore`、`get_proxy_status`、`is_proxy_running`、`get_proxy_config`、`update_proxy_config`、`get_global_proxy_config`、`update_global_proxy_config`、`set_global_proxy_url`、`get_proxy_config_for_app`、`set_proxy_takeover_for_app`、`get_proxy_takeover_status`、`is_live_takeover_active`。 |
| **故障转移 (Failover)** | ✅ 完全支持 | `get_failover_queue`、`add_to_failover_queue`、`remove_from_failover_queue`、`get_auto_failover_enabled`、`set_auto_failover_enabled`、`get_available_providers_for_failover`。直接操作 providers 表。 |
| **流检测 (Stream Check)** | ✅ 完全支持 | `stream_check_provider`、`stream_check_all_providers`、`get_stream_check_config`、`save_stream_check_config`。本地 HTTP 探测(Anthropic 风格 `/v1/messages`),足以驱动 Web UI。 |
| **MCP 服务器** | ✅ 完全支持 | 统一接口 `get_mcp_servers`、`upsert_mcp_server`、`delete_mcp_server`、`toggle_mcp_app`、`set_mcp_enabled`、`import_mcp_from_apps`,以及兼容的逐应用版本 `upsert_mcp_server_in_config` / `delete_mcp_server_in_config`。全部走 `McpService`。 |
| **Claude MCP** | ✅ 完全支持 | `get_claude_mcp_status`、`read_claude_mcp_config`、`upsert_claude_mcp_server`、`delete_claude_mcp_server`、`validate_mcp_command` —— 通过上游 re-export 的命令读写 `~/.claude.json`。 |
| **提示词 (Prompts)** | ✅ 完全支持 | `get_prompts`、`upsert_prompt`、`delete_prompt`、`enable_prompt`、`import_prompt_from_file`、`get_current_prompt_file_content`。直接 SQL 操作 `prompts` 表(上游 `Prompt` 类型为私有)。 |
| **技能 (Skills)** | ⚠️ 主要为只读 | `get_installed_skills`、`get_skill_backups`、`delete_skill_backup`、`toggle_skill_app`、`scan_unmanaged_skills`、`migrate_skill_storage`、`uninstall_skill_unified`、`uninstall_skill`。安装 / 更新 / 搜索的路径被桩化(因为 `DiscoverableSkill`、`SkillRepo`、`SkillUpdateInfo` 等类型为私有);可通过上游桌面版安装,然后在 Web UI 中查看结果。 |
| **用量统计** | ⚠️ 只读 | `get_usage_summary`、`get_usage_summary_by_app`、`get_usage_trends`、`get_provider_stats`、`get_model_stats`、`get_request_logs`、`get_request_detail`、`check_provider_limits`、`get_model_pricing`、`update_model_pricing`、`delete_model_pricing`。`sync_session_usage` 是桩实现(`services::session_usage::*` 为私有),真实同步请使用上游桌面端。 |
| **会话 (Sessions)** | ✅ 完全支持 | `list_sessions`、`get_session_messages`、`delete_session`、`delete_sessions` 通过 re-export 转发。`launch_session_terminal` 是桩(服务器无终端 —— 用户需在主机上手动启动 AI CLI)。 |
| **Hermes** | ✅ 完全支持 | `get_hermes_live_provider_ids`、`get_hermes_live_provider`、`import_hermes_providers_from_live`、`get_hermes_model_config`、`open_hermes_web_ui`、`launch_hermes_dashboard`、`get_hermes_memory`、`set_hermes_memory`、`get_hermes_memory_limits`、`set_hermes_memory_enabled`。 |
| **OpenClaw 在线编辑** | ✅ 完全支持 | `get_openclaw_live_provider_ids`、`get_openclaw_live_provider`、`import_openclaw_providers_from_live`、`scan_openclaw_config_health`,以及 `agents.defaults` / `env` / `tools` / `model` / `models` 编辑器。`openclaw.json` 由我们直接读写(上游模块为私有)。 |
| **OpenCode** | ✅ 完全支持 | `get_opencode_live_provider_ids`、`import_opencode_providers_from_live`。 |
| **OMO** | ⚠️ 只读 | `read_omo_local_file`、`read_omo_slim_local_file`、`get_current_omo_provider_id`、`get_current_omo_slim_provider_id`、`disable_current_omo`、`disable_current_omo_slim`。 |
| **深链 (Deeplink)** | ⚠️ 主要为只读 | `parse_deeplink`、`merge_deeplink_config`、`import_from_deeplink_unified`(仅 provider 路径)。MCP / Skill 深链导入返回错误,引导用户使用 `import_mcp_from_apps` 或上游桌面端。 |
| **数据库备份** | ✅ 完全支持 | `create_db_backup`、`list_db_backups`、`delete_db_backup`、`restore_db_backup`、`rename_db_backup` —— 文件复制实现。 |
| **S3 / WebDAV 同步** | ⚠️ 仅设置项 | `s3_sync_save_settings` / `webdav_sync_save_settings` 把配置写入 settings 表。`*_sync_upload` / `*_sync_download` / `*_test_connection` / `*_sync_fetch_remote_info` 均为桩(`services::s3_sync`、`services::webdav_sync` 为私有)。真实传输请使用上游桌面端。 |
| **Codex OAuth** | ⚠️ 桩 | `get_codex_oauth_quota`、`get_codex_oauth_models` 返回 `null`(OAuth 管理器内嵌于 Tauri 运行时)。请使用上游桌面端进行 OAuth 配对。 |
| **GitHub / Copilot 通用认证** | ⚠️ 桩 | `auth_get_status`、`auth_list_accounts`、`auth_logout`、`auth_remove_account`、`auth_set_default_account`、`copilot_*` 及其账户变体均返回确定性的空状态,OAuth 流程需在桌面端完成。 |
| **桌面端独占命令** | ⚠️ 桩 | `open_external` 仅记录 URL(无浏览器可弹窗);`open_file_dialog` / `save_file_dialog` / `open_zip_file_dialog` / `pick_directory` / `open_app_config_folder` / `open_config_folder` / `open_provider_terminal` / `open_workspace_directory` / `set_window_theme` / `restart_app` / `set_auto_launch` / `is_portable_mode` / `get_auto_launch_status` / `check_for_updates` / `update_tray_menu` —— 桌面或窗口管理命令。 |
| **会话日志同步 / 实时统计** | ⚠️ 桩 | `sync_session_usage`、`queryProviderUsage`、`testUsageScript`、`test_api_endpoints` —— 都需要运行中的代理服务,而本程序不提供代理。 |
| **自定义端点追踪** | ⚠️ 桩 | `get_custom_endpoints`、`add_custom_endpoint`、`remove_custom_endpoint`、`update_endpoint_last_used` 接收调用并确认,但底层 `Provider` 字段为私有,需要更深的反射支持。 |
| **Claude Desktop** | ⚠️ 桩 | `get_claude_desktop_status` 仅探测文件系统;`get_claude_desktop_default_routes`、`import_claude_desktop_providers_from_claude`、`ensure_claude_desktop_official_provider` 为桩实现。 |

### “桩实现”是如何工作的?

cc-switch 前端把缺失的命令视为 404 并回退到 “该功能不可用” 状态。cc-switch-web 返回空值(`[]`、`null`、`false`)或日志消息,前端不会崩溃,只是隐藏对应按钮。这与项目 “不写业务逻辑” 的契约一致 —— dispatch 层是瘦壳。

凡上游 API 把类型放在 `pub mod` 下,我们直接接进来;如果是私有模块,就操作底层数据库 / 文件。详见 `.ccsm/server/src/commands_extra/*.rs` 顶部说明。

## 日常使用

服务器启动后,Web UI 即可通过安装时选择的地址访问。日常操作通过两个入口暴露:一个 CLI 包装脚本和 Web UI 本身。

### 服务管理

包装脚本安装在 `~/.local/bin/cc-switch-web-ctl`:

| 子命令 | 作用 |
| --- | --- |
| `cc-switch-web-ctl start` | 启用并启动 user-systemd 服务 |
| `cc-switch-web-ctl stop` | 停止服务 |
| `cc-switch-web-ctl restart` | 优雅重启 (`update` 后自动调用) |
| `cc-switch-web-ctl status` | 一行打印服务状态、监听端口、公网端点、FRP 隧道 |
| `cc-switch-web-ctl logs` | `journalctl --user -u cc-switch-web -f` |
| `cc-switch-web-ctl update` | `git pull` + `pnpm install` + `cargo build --release` + 重装 + 重启 |

`status` 一次输出:

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

### Web UI 操作

浏览器打开 `https://<你的域名>/`。如果启用了 Basic Auth(见 `.ccsm/deploy/README.md` 的 **Configuration** 章节),会先弹认证窗口。UI 本身即上游 React 应用,未做任何修改。

| UI 操作 | 后端效果 |
| --- | --- |
| 切换应用 Tab (Claude Code / Codex / Hermes / ...) | 通过 `/api/invoke/get_providers` 重新拉取该应用的 provider 列表 |
| 添加 Provider(➕ 按钮) | 弹出预设目录;选中一个后 POST `/api/invoke/add_provider` 并携带渲染好的 `Provider` 对象 |
| 启用(启用)某个 provider | POST `/api/invoke/switch_provider` → 上游 `ProviderService::switch` 重写 `~/.claude/settings.json` (或对应文件) 并发出 `provider-switched` SSE 事件 |
| 编辑 / 删除 / 复制 URL | 标准的 `/api/invoke/update_provider` / `delete_provider` / `read_live_provider_settings` 调用 |
| 点击齿轮图标(设置) | 6 个 Tab(通用 / 路由 / 认证 / 高级 / 使用统计 / 关于)各自对应一组 `get_*` / `save_*` 命令 |
| MCP / 提示词 / 技能 | 同样的模式 —— 每个 Tab 都是一个轻量 `invoke` 包装 |

完整的 dispatch 引用(哪个 UI 命令对应哪个服务方法、哪些在无头服务器上是桩实现)请见下方的 **与上游功能的对齐情况** 章节。

### 通过 SSH 隧道访问 Web UI(不绑定域名时)

如果不在公网暴露服务,推荐先用 SSH 隧道:

```bash
ssh -L 3000:localhost:3000 user@host
# 然后浏览器打开 http://localhost:3000
```

服务默认绑定 `127.0.0.1:3000`,这是开发者机器上的推荐访问方式。

### 数据存放位置

cc-switch-web 完整沿用上游目录布局。你可以在桌面版与本服务之间无缝切换,无需导入导出:

| 内容 | 路径 | 所有者 |
| --- | --- | --- |
| SQLite 数据库 | `~/.cc-switch/cc-switch.db` | 双方(上游 schema) |
| Hermes 配置 | `~/.hermes/config.yaml` | 双方 |
| Claude 配置 | `~/.claude/settings.json` | 双方 |
| Codex 配置 | `~/.codex/config.toml` | 双方 |
| Gemini 配置 | `~/.gemini/config.json` | 双方 |
| 日志 / 临时文件 | `~/.local/share/cc-switch-web/` | 仅 cc-switch-web |

## 架构

```
+-------------------+        +-----------------------------+
| 浏览器            |  HTTP  | cc-switch-web              |
|                   |  ----> |                             |
|  - React SPA      |        |  Axum 路由                  |
|  - bridge/*       |        |    POST /api/invoke/<cmd>   |
|    (替换掉        |  SSE   |    GET  /api/events         |
|     @tauri-apps/  | <----  |    GET  /api/health         |
|     api/*)        |        |    GET  /<file>  (SPA)      |
|                   |        |                             |
|                   |        |  dispatch  ->  cc_switch_lib (上游)
|                   |        |              - ProviderService
|                   |        |              - ProxyService
|                   |        |              - ConfigService
|                   |        |              - McpService
|                   |        |              - StreamCheckService
|                   |        |              - Database (SQLite)
+-------------------+        +-----------------------------+
```

桥接层是一个薄薄的 TypeScript 层:`@tauri-apps/api/core` 的 `invoke` 变成 `POST /api/invoke/<cmd>`,`@tauri-apps/api/event` 的 `listen` 变成 `GET /api/events` SSE 消费者。其余 `@tauri-apps/*` 包 (`window`、`app`、`path`、`plugin-dialog`、`plugin-process`、`plugin-store`、`plugin-updater`) 是桩或轻薄回退。

## 开发工作流

| 任务 | 命令 |
| --- | --- |
| 启动开发服务 | `cargo run -p cc-switch-web-server` |
| 启动到指定端口 | `cargo run -p cc-switch-web-server -- --port 8080` |
| 前端类型检查 | `pnpm run typecheck` |
| 打包发布版 | `pnpm run build:renderer && cargo build --release -p cc-switch-web-server` |
| 验证 dispatch 覆盖率 | `bash .ccsm/scripts/check-coverage.sh` |
| 运行单元测试 | `cargo test -p cc-switch-web-server` |
| 同步上游(维护者) | `bash .ccsm/scripts/sync-upstream.sh` |

## 同步策略

本仓库是上游的 fork。`.ccsm/scripts/sync-upstream.sh` 执行 `git fetch && git merge upstream/main` 并按以下冲突解决策略:

1. 接受上游原版 `src/` 和 `src-tauri/`。
2. 保留我们的 `.ccsm/`、根 `Cargo.toml`,以及 `package.json` 里的 `pnpm.overrides`。

`Cargo.toml` / `package.json` 中的版本号始终对齐上游 tag;安装脚本和发布流程拉取同 tag 对应的二进制。

## 协议

MIT(继承自上游)。
