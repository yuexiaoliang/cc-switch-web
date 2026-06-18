# cc-switch-web 项目文档

> 为无桌面环境的 Linux 服务器提供 cc-switch 的核心功能， 以 Web 界面远程管理 Provider 配置和代理转发。

---

## 一、项目概述

### 1.1 背景

[cc-switch](https://github.com/farion1231/cc-switch) 是一个广受欢迎的 AI 工具配置管理器， 支持 Claude Code、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes 等工具的多 Provider 管理和一键切换。

### 为什么需要 cc-switch？

**核心功能：多 Provider 一键切换**

用户通常同时使用多个 AI 供应商（OpenAI、Anthropic、DeepSeek、硅基流动等），每个工具（Claude Code、Codex、Gemini CLI、OpenCode、OpenClaw、Hermes 等）都需要单独配置 API Key 和模型。cc-switch 提供集中管理：

- **统一管理**：在一个界面中维护所有 Provider 的 API Key、Base URL、模型配置
- **一键切换**：选择目标 Provider 后，自动将该工具的系统配置文件（`~/.claude/settings.json`、`~/.codex/config.toml`、`~/.gemini/config.json` 等）更新为对应配置
- **预设模板**：内置常见 Provider 模板，用户只需填写 API Key 即可使用

**增强功能：协议转换代理**

部分工具与第三方 Provider 存在协议不兼容。例如 Codex CLI 使用 OpenAI Responses API，而部分国内 Provider 只支持 Chat Completions API。cc-switch 通过本地代理服务，在请求/响应层面实时转换协议格式。

**安全隔离**：工具配置文件中只存储占位 API Key，真实 Key 由 cc-switch 保管并在转发时动态注入。

### 为什么需要 cc-switch-web？

cc-switch 基于 Tauri（Rust + WebView）构建，**需要桌面图形环境**。 大量用户在日常使用的**无头 Linux 服务器**（SSH 远程、WSL、Docker、云主机） 上无法运行 cc-switch，只能手动编辑各工具的配置文件，无法享受一键切换的便利。

### 1.2 项目定位

**cc-switch-web** 是 cc-switch 的**服务端/Web 衍生版本**，目标：

- 在无桌面环境的 Linux 服务器上运行
- 通过浏览器远程访问管理界面
- 提供 cc-switch 的**核心功能**：Provider 管理、切换、代理转发
- **直接复用上游代码**，只做最小适配层

### 1.3 与上游的关系

| 维度 | cc-switch（上游） | cc-switch-web（本项目） |
| --- | --- | --- |
| 运行环境 | 桌面系统（Mac/Win/Linux GUI） | 无头 Linux 服务器 |
| 界面 | Tauri WebView | 浏览器远程访问 |
| 目标用户 | 桌面开发者 | 服务器用户、WSL 用户 |
| 代码策略 | 原版 | Fork + 适配层，复用上游业务代码 |
| 功能范围 | 全功能 | 核心功能子集 |
| **版本号** | 自主发布 | **永远跟随上游版本号** |

---

## 二、核心需求

### 2.1 功能性需求

1. **Provider 管理**

   - 增删改查 Provider 配置（API Key、Base URL、模型名等）
   - 内置预设模板（DeepSeek、硅基流动、OpenAI 等）
   - Provider 拖拽排序

2. **Provider 切换**

   - 一键切换当前活跃 Provider
   - 自动将配置写入目标工具的系统配置文件
   - 支持 Claude Code、Codex 等工具

3. **代理转发**

   - 启动 HTTP 代理服务（协议转换：如 Codex Responses API → Chat Completions）
   - 代理配置管理
   - 启动/停止控制

4. **流检测**

   - 测试 Provider API 连通性
   - 显示延迟和状态

5. **基础设置**

   - 主题切换
   - 语言切换
   - 配置导入导出（JSON 备份）

### 2.2 非功能性需求

1. **零修改上游业务代码**

   - `src/`（前端 React 代码）一字符不改
   - `src-tauri/src/`（Rust 业务逻辑）一字符不改
   - 只新增适配层和修改构建配置

2. **上游同步即时生效**

   - 上游更新 UI 组件 → 同步后立刻生效
   - 上游更新业务逻辑 → 同步后立刻生效
   - 上游新增 command → 需后端添加路由（1 行）

3. **最小用户心智成本**

   - 复用上游完整的 Web UI，用户零学习成本
   - 安装启动极简：下载一个二进制，执行即运行

4. **单机单用户**

   - 无需登录、多用户隔离、权限系统
   - 数据存储在本地 SQLite

---

## 三、目标用户与使用场景

### 3.1 典型用户

- 在远程云服务器（AWS、GCP、阿里云）上开发的用户
- WSL2 用户（Windows 子系统无 GUI）
- Docker 容器内使用 AI CLI 工具的用户
- 本地 Linux 工作站偏好浏览器管理而非桌面应用的用户

### 3.2 典型使用流程

```
1. SSH 登录服务器
   ssh user@server

2. 安装 cc-switch-web
   curl -fsSL https://raw.githubusercontent.com/yuexiaoliang/cc-switch-web/main/.ccsm/scripts/install.sh | sh

3. 启动服务（默认只监听本地，安全）
   cc-switch-web
   # 🚀 CCSwitch Mini running at http://127.0.0.1:3000

4. 本地浏览器访问（通过 SSH 隧道）
   # 在本地终端执行，将服务器的 3000 端口映射到本地
   ssh -L 3000:localhost:3000 user@server
   # 打开 http://localhost:3000

5. 在 Web 界面中：
   - 添加 DeepSeek Provider（填 API Key）
   - 切换到 DeepSeek（自动写入 ~/.codex/config.toml 等系统配置）
   - 如需协议转换，启动代理服务

6. 直接使用各 AI 工具
   codex        # 自动使用当前 Provider
   claude       # 自动使用当前 Provider
   # 各工具自动读取已写入的系统配置
```

---

## 四、设计约束与考量

### 4.1 技术约束

浏览器安全沙箱禁止写入本地文件系统，无法修改 `~/.codex/config.toml`、 `~/.claude/settings.json` 等系统配置文件，也无法启动本地代理服务。 必须在服务器上运行一个**有文件系统权限的本地进程**。

### 4.2 功能边界（不做的事）

| 功能 | 不做原因 |
| --- | --- |
| Claude Desktop 配置写入 | 服务器通常没有 Claude Desktop |
| 系统代理设置 | 无桌面环境，无系统代理概念 |
| 开机自启 | 用 systemd 用户自己配 |
| 托盘菜单 | 无桌面环境 |
| MCP 本地进程管理 | 服务器场景暂不需要 |
| Copilot OAuth | 非核心，增加复杂度 |
| Skills/Prompts 管理 | 服务器场景暂不需要 |
| 故障转移 | 可后期加，MVP 不做 |
| ~~用量精确计费~~ | ~~不准确，不做~~ |

---

## 五、架构方案

### 5.1 仓库结构

```
cc-switch-web/                  # GitHub 仓库（fork 自 farion1231/cc-switch）
│
├── src/                         # ← 上游前端代码（fork 而来，不修改）
│   ├── components/              #   UI 组件
│   ├── lib/api/                 #   API 调用层
│   ├── hooks/                   #   React Hooks
│   └── ...                      #   所有前端业务代码
│
├── src-tauri/src/               # ← 上游 Rust 代码（fork 而来，不修改）
│   ├── services/                #   ProviderService / ProxyService
│   ├── database/                #   SQLite 数据库层
│   ├── proxy/                   #   HTTP 代理转发逻辑
│   ├── provider.rs              #   Provider 模型
│   ├── settings.rs              #   设置模型
│   └── ...                      #   所有 Rust 业务代码
│
├── .ccsm/                       # 新增：本项目专属模块（点前缀防冲突）
│   ├── bridge/                  #   前端 Tauri API → HTTP 桥接
│   │   ├── core.ts              #     invoke → fetch
│   │   ├── event.ts             #     listen → SSE
│   │   ├── window.ts            #     getCurrentWindow → mock
│   │   ├── path.ts              #     appConfigDir → 固定路径
│   │   ├── plugin-dialog.ts     #     message → alert
│   │   ├── plugin-process.ts    #     exit → reload
│   │   ├── plugin-store.ts      #     Store → localStorage
│   │   └── package.json         #     伪装 @tauri-apps/api
│   │
│   ├── server/                  #   Rust HTTP 适配层
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── main.rs          #     Axum 启动 + 静态文件服务
│   │       ├── handlers.rs      #     HTTP → Service dispatch
│   │       └── state.rs         #     AppState 初始化
│   │
│   └── scripts/                 #   维护脚本
│       ├── install.sh           #     安装脚本
│       └── sync-upstream.sh     #     同步上游脚本（git merge）
│
├── package.json                 # ← 修改：添加 npm overrides
├── Cargo.toml                   # ← 修改：添加 .ccsm/server workspace 成员
├── vite.config.ts               # ← 上游（可选修改）
└── README.md                    # ← 覆盖为项目文档
```

### 5.2 运行时架构

```
用户浏览器 ──HTTP──→ cc-switch-web 进程
                        │
                        ├── Axum Router
                        │   ├── /api/invoke/*  → handlers.rs dispatch
                        │   └── /*             → 前端静态文件
                        │
                        ├── handlers.rs
                        │   └── 根据 cmd 字符串分发到上游 Service
                        │
                        ├── 上游 Service 层（复用）
                        │   ├── ProviderService::list/add/update/delete/switch
                        │   ├── ProxyService::start/stop/get_config
                        │   ├── StreamCheckService::check
                        │   └── ConfigService::get/save_settings
                        │
                        ├── 上游 Database 层（复用）
                        │   └── SQLite（~/.local/share/cc-switch-web/）
                        │
                        └── 系统配置文件写入
                            ├── ~/.codex/config.toml
                            ├── ~/.codex/auth.json
                            └── ~/.claude/settings.json
```

### 5.3 前端桥接层

Vite 构建时，`@tauri-apps/api/*` 的导入被替换为 `.ccsm/bridge/*`：

```ts
// src/lib/api/settings.ts（上游代码，不修改）
import { invoke } from "@tauri-apps/api/core";   // ← 被替换为 .ccsm/bridge/core.ts

export const settingsApi = {
  async get(): Promise<Settings> {
    return await invoke("get_settings");         // ← 实际发 HTTP POST /api/invoke/get_settings
  }
};
```

替换机制：`package.json` 中的 `overrides`：

```json
{
  "overrides": {
    "@tauri-apps/api": "file:./.ccsm/bridge",
    "@tauri-apps/plugin-dialog": "file:./.ccsm/bridge",
    "@tauri-apps/plugin-process": "file:./.ccsm/bridge",
    "@tauri-apps/plugin-store": "file:./.ccsm/bridge"
  }
}
```

### 5.4 后端 Dispatch 层

每个 command 一个分支，调用上游 Service，无业务逻辑：

```rust
// .ccsm/server/src/handlers.rs
async fn dispatch(Json(req): Json<InvokeRequest>) -> Result<Json<Value>, String> {
    match req.cmd.as_str() {
        "get_providers" => {
            let app = req.args["app"].as_str().ok_or("missing app")?;
            let app_type = AppType::from_str(app).map_err(|e| e.to_string())?;
            let result = ProviderService::list(&STATE, app_type)
                .map_err(|e| e.to_string())?;
            Ok(Json(serde_json::to_value(result).unwrap()))
        }
        // ... 每个 command 3-5 行
        _ => Err(format!("Unknown command: {}", req.cmd)),
    }
}
```

---

## 六、核心 Command 覆盖清单

### 6.1 必须实现（P0）

| Command | 上游 Service | 说明 |
| --- | --- | --- |
| `get_providers` | `ProviderService::list` | Provider 列表 |
| `get_current_provider` | `ProviderService::current` | 当前 Provider |
| `add_provider` | `ProviderService::add` | 添加 Provider |
| `update_provider` | `ProviderService::update` | 更新 Provider |
| `delete_provider` | `ProviderService::delete` | 删除 Provider |
| `switch_provider` | `ProviderService::switch` | 切换 Provider |
| `update_providers_sort_order` | `ProviderService::update_sort` | 排序 |
| `import_default_config` | `ProviderService::import_default` | 导入默认配置 |
| `get_settings` | `ConfigService::get_settings` | 获取设置 |
| `save_settings` | `ConfigService::save_settings` | 保存设置 |
| `start_proxy_server` | `ProxyService::start` | 启动代理 |
| `stop_proxy_with_restore` | `ProxyService::stop` | 停止代理 |
| `get_proxy_status` | `ProxyService::get_status` | 代理状态 |
| `get_proxy_config` | `ProxyService::get_config` | 代理配置 |
| `update_proxy_config` | `ProxyService::update_config` | 更新代理配置 |
| `is_proxy_running` | `ProxyService::is_running` | 是否运行中 |
| `stream_check_provider` | `StreamCheckService::check` | 单 Provider 检测 |
| `stream_check_all_providers` | `StreamCheckService::check_all` | 全部检测 |
| `get_stream_check_config` | `StreamCheckService::get_config` | 检测配置 |
| `save_stream_check_config` | `StreamCheckService::save_config` | 保存检测配置 |
| `open_external` | \- | 打开外部链接（浏览器 window.open） |
| `get_config_dir` | \- | 返回配置目录路径 |
| `get_app_config_path` | \- | 返回应用配置路径 |
| `get_tool_versions` | \- | 返回版本信息 |

### 6.2 暂不提供（上游调用但返回固定值）

| Command | 处理方式 |
| --- | --- |
| `get_auto_launch_status` | 固定返回 `false` |
| `set_auto_launch` | 空操作 |
| `is_portable_mode` | 固定返回 `false` |
| `restart_app` | 空操作 |
| `check_for_updates` | 空操作 |
| `update_tray_menu` | 空操作 |
| `open_app_config_folder` | 空操作（服务器无文件管理器） |
| `open_config_folder` | 空操作 |
| `open_file_dialog` | 返回 `null` |
| `save_file_dialog` | 返回 `null` |

### 6.3 未使用（前端不调用，无需实现）

上游 `src/lib/api/` 中大量 commands 涉及的功能不在 MVP 范围内： Auth、Copilot、Hermes、OpenClaw、OpenCode、MCP、Skills、Prompts、Sessions、Workspace、S3 同步、WebDAV 同步等。

---

## 七、上游同步策略

### 7.1 同步方式：Fork + Merge

本项目是 [cc-switch](https://github.com/farion1231/cc-switch) 的 fork，代码结构和上游完全一致。 唯一差异是新增了 `.ccsm/` 目录存放我们的适配层代码。

```bash
# 配置上游 remote
git remote add upstream https://github.com/farion1231/cc-switch.git
git fetch upstream

# 合并上游最新代码
git merge upstream/main
```

**原则**：

- 仓库是上游的 fork，目录结构和上游完全一致
- 只新增 `.ccsm/` 目录，不修改上游原有文件
- 同步是**全量合并**，永远完整包含上游最新代码

### 7.2 冲突处理

合并后如发生冲突，按以下规则处理：

| 冲突文件 | 处理方式 |
| --- | --- |
| `src/**` | 以**上游为准**，完全接受 upstream 版本 |
| `src-tauri/src/**` | 以**上游为准**，完全接受 upstream 版本 |
| `package.json` | 接受 upstream 版本，**保留我们的** `overrides` **行** |
| `.ccsm/**` | 保留**我们的**版本 |
| 其他新增冲突 | 评估后决定 |

```bash
# 快捷处理：自动接受 upstream 的 src/ 和 src-tauri/src/
git checkout --theirs src/ src-tauri/src/
git add src/ src-tauri/src/

# 保留我们的 .ccsm/
git checkout --ours .ccsm/
git add .ccsm/
```

### 7.3 同步后检查清单

```bash
# 1. 检查上游是否新增 invoke 调用（前端调了但后端没实现）
.ccsm/scripts/check-coverage.sh

# 2. 编译检查（Rust 类型系统会暴露接口变更）
cargo check -p cc-switch-web-server

# 3. 前端构建检查
npm install && npm run build

# 4. 手动回归核心流程
#    - 添加 Provider → 切换 → 写入系统配置
#    - 启动代理 → 请求转发
```

---

## 八、分发与部署

### 8.1 安装方式

**方式一：一键脚本（推荐）**

```bash
curl -fsSL https://raw.githubusercontent.com/yuexiaoliang/cc-switch-web/main/.ccsm/scripts/install.sh | sh
# 下载对应平台二进制到 /usr/local/bin/
```

**方式二：手动下载**

```bash
wget https://github.com/.../cc-switch-web-linux-x64
chmod +x cc-switch-web-linux-x64
sudo mv cc-switch-web-linux-x64 /usr/local/bin/cc-switch-web
```

**方式三：cargo install（有 Rust 环境）**

```bash
cargo install cc-switch-web
```

### 8.2 启动

```bash
cc-switch-web
# 🚀 CCSwitch Mini running at http://127.0.0.1:3000
#
# 💡 SSH 隧道:   ssh -L 3000:localhost:3000 user@server
```

### 8.3 安全配置（重要）

**默认行为**：绑定 `127.0.0.1`，仅本机可访问。这是推荐且安全的方式。

```bash
# 默认：安全，仅本机访问
cc-switch-web

# SSH 隧道访问（推荐）
ssh -L 3000:localhost:3000 user@server
```

**如需公网访问**（不推荐，风险自负）：

```bash
# 显式绑定所有接口（Provider 配置和 API Key 将暴露给任何能访问该 IP 的人）
cc-switch-web --host 0.0.0.0

# 建议配合反向代理 + Basic Auth 或 VPN 使用
```

**其他选项**：

```bash
# 自定义端口
cc-switch-web --port 8080

# 数据目录
cc-switch-web --data-dir /var/lib/cc-switch-web
```

### 8.4 构建流程

```bash
# 1. 前端构建
npm install
npm run build        # → dist/

# 2. 后端构建（嵌入前端 dist/）
cargo build --release -p cc-switch-web-server

# 3. 产物：单个二进制文件
# target/release/cc-switch-web-server
```

---

## 九、技术栈

| 层级 | 技术 | 说明 |
| --- | --- | --- |
| 前端 | React + Vite + Tailwind + shadcn/ui | 上游代码，直接复用 |
| 前端桥接 | TypeScript（.ccsm/bridge） | 替换 Tauri API 为 HTTP |
| 后端框架 | Axum（Rust） | HTTP server + 路由 |
| 业务逻辑 | 复用上游 Rust（services、database、proxy） | 不复写 |
| 数据存储 | SQLite | 复用上游 database 模块 |
| 代理转发 | 复用上游 proxy 模块 | Rust 原生实现 |
| 构建 | Cargo + npm | Rust 编译 + Vite 构建 |

---

## 十、版本号策略

**原则：永远跟随上游版本号。**

- 上游发布 `v3.16.2`，本项目同步后发布 `v3.16.2`
- 用户看到版本号即可知道对应上游的哪个版本
- 如果适配层有独立更新但上游未发新版，使用 `v3.16.2+ccs.1` 格式（SemVer build metadata）

**版本号来源**：从上游 `package.json` 和 `Cargo.toml` 中读取，`cc-switch-web` 不做独立版本号管理。

---

## 十一、风险与应对

| 风险 | 影响 | 应对 |
| --- | --- | --- |
| 上游新增大量 commands | 后端需补充 dispatch | check-coverage.sh 自动检测 |
| 上游修改 Service 接口签名 | 编译失败 | Rust 类型系统保障，编译时发现问题 |
| 上游 Tauri 依赖增强 | 桥接层需补充 mock | 按需补充 bridge 模块 |
| ProxyService 依赖 AppHandle | 故障转移事件丢失 | 不设置 app_handle，事件静默 |
| 前端调用未注册 command | 运行时 404 | 开发时 check-coverage 捕获 |
