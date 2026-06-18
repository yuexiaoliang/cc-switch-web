# 上游兼容性修改说明

## 修改目标

确保 cc-switch-web 与上游 Tauri 应用使用完全相同的文件存储位置，这样用户在两者之间切换时不需要任何数据迁移。

## 修改内容

### 1. 移除 `CC_SWITCH_TEST_HOME` 环境变量设置

**文件**: `.ccsm/server/src/state.rs`

**修改前**:
- 设置 `CC_SWITCH_TEST_HOME` 环境变量为 `--data-dir` 参数值
- 所有配置文件（Hermes、Claude、Codex、Gemini）和数据库都被重定位到 `<data-dir>/` 下

**修改后**:
- 不再设置 `CC_SWITCH_TEST_HOME` 环境变量
- 所有配置文件使用上游标准位置

### 2. 文件存储位置对比

| 文件类型 | 修改前位置 | 修改后位置（上游标准） |
|---------|----------|---------------------|
| 数据库 | `<data-dir>/.cc-switch/cc-switch.db` | `~/.cc-switch/cc-switch.db` |
| Hermes 配置 | `<data-dir>/.hermes/config.yaml` | `~/.hermes/config.yaml` |
| Hermes 记忆 | `<data-dir>/.hermes/memories/` | `~/.hermes/memories/` |
| Claude 配置 | `<data-dir>/.claude/` | `~/.claude/` |
| Codex 配置 | `<data-dir>/.codex/` | `~/.codex/` |
| Gemini 配置 | `<data-dir>/.gemini/` | `~/.gemini/` |

其中 `<data-dir>` 默认为 `~/.local/share/cc-switch-web/`

### 3. 保留的功能

- `--config-dir` 参数仍然可用，用于覆盖宿主工具（Claude/Codex/Gemini）的配置目录
- `--data-dir` 参数保留用于向后兼容，但现在仅影响日志和临时文件（如果有）
- `--port`、`--host`、`--token` 等服务器参数不受影响

## 验证结果

```bash
# 验证文件位置
ls ~/.hermes/config.yaml          # ✓ 存在
ls ~/.hermes/memories/MEMORY.md   # ✓ 存在
ls ~/.cc-switch/cc-switch.db      # ✓ 存在
ls ~/.claude/                     # ✓ 存在
```

## 迁移说明

如果用户之前使用的是旧版本的 cc-switch-web（数据在 `<data-dir>/` 下），需要手动迁移：

```bash
# 迁移数据库
cp ~/.local/share/cc-switch-web/.cc-switch/cc-switch.db ~/.cc-switch/

# 迁移 Hermes 配置
cp -r ~/.local/share/cc-switch-web/.hermes/* ~/.hermes/

# 迁移 Claude 配置（如果有）
cp -r ~/.local/share/cc-switch-web/.claude/* ~/.claude/

# 以此类推...
```

**注意**: 如果 `~/.cc-switch/` 或 `~/.hermes/` 等目录已经存在文件，请先备份后再覆盖。

## 优势

1. **无缝迁移**: 用户可以在 cc-switch-web 和上游 Tauri 应用之间自由切换
2. **符合预期**: 配置文件位置与上游文档一致
3. **易于备份**: 所有配置都在用户主目录下的标准位置
4. **多版本共存**: 不会因为是 cc-switch-web 还是上游版本而产生冲突

## 技术细节

上游代码通过 `CC_SWITCH_TEST_HOME` 环境变量来支持测试场景下的目录重定位。cc-switch-web 之前滥用了这个机制来实现数据隔离，但这导致了与上游的不兼容。

现在 cc-switch-web 直接使用上游的标准路径解析逻辑，确保行为完全一致。
