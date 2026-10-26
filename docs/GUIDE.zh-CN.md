# Hangar 技术指南

> 面向开发者和进阶用户：架构、构建、CLI、MCP、数据说明。
> 普通使用请看[说明书（人人能懂版）](说明书.md)。

## 技术栈与架构

**一个核心，两个前端**：

```
crates/core    Rust 核心库：数据模型、SQLite（rusqlite, WAL）、导入扫描、
               git 状态（git2）、终端/agent 启动（launcher）——无 UI 依赖
crates/cli     hangar 命令行 + MCP 服务器（clap + 手写 stdio JSON-RPC）
src-tauri      Tauri 2 桌面壳：commands 薄层，全部逻辑下沉 core
src/           Vue 3 + Vite + TypeScript + Tailwind v4 前端
```

GUI 与 CLI 读写**同一个 SQLite 文件**（WAL + 5s busy_timeout），可并发使用。

## 从源码构建

```bash
git clone <repo> && cd hangar
npm install
npm run tauri dev      # 开发模式（改 Rust 自动重编重启，改前端 HMR）
npm run tauri build    # 出 NSIS 安装包（target/release/bundle/nsis/）
cargo build -p hangar-cli --release   # 单独出 CLI
```

要求：Node 20+、Rust stable（MSVC）、WebView2（Win10/11 自带）。

已知开发坑：

- vite watcher 必须忽略 `**/target/**`（cargo 写 DLL 时文件锁会让 vite EBUSY 崩掉），已在 `vite.config.ts` 配好
- 同时跑 `tauri dev` 和 `tauri build` 会抢 cargo 构建锁，排队属正常
- 停 dev 后 vite 子进程可能残留占 5173 端口：`netstat -ano | findstr :5173` 找 PID 杀掉

## 命令行（CLI）

```bash
hangar ls [query]                     # 列出（模糊匹配名称/路径/标签）
hangar add <path> [--name] [--agent] [--tag…]
hangar open <query> [--terminal]      # 资源管理器 / 默认终端
hangar run <query> [--config NAME]    # 运行启动配置（默认第一条）
hangar star <query>                   # 收藏/取消
hangar tag <query> <tags…>            # 追加标签
hangar rm <query>                     # 软删除（同路径重新登记会复活记录）
```

匹配规则：精确名称/路径 > 唯一包含匹配 > 多个匹配时列出候选要求更精确。

## REST API（HTTP 接入）

应用启动时（或运行 `hangar serve`）自动监听 `http://127.0.0.1:7333`，仅本机可达。所有请求需带 `Authorization: Bearer <token>`（token 在设置页可见，或 `hangar config get rest_token`；仅 `/health` 免鉴权）。

```bash
# 列出（可 ?query= 过滤）
curl -H "Authorization: Bearer $TOKEN" http://127.0.0.1:7333/workspaces
# 详情（含 git 状态与启动配置）
curl -H "Authorization: Bearer $TOKEN" "http://127.0.0.1:7333/workspace?query=博客"
# 登记 / 打开 / 启动 / 记工作动态
curl -X POST -H "Authorization: Bearer $TOKEN" -d '{"path":"D:/work/x","name":"x"}' .../workspaces
curl -X POST -H "Authorization: Bearer $TOKEN" -d '{"query":"x","via":"terminal"}' .../open
curl -X POST -H "Authorization: Bearer $TOKEN" -d '{"query":"x","agent":"claude"}' .../launch
curl -X POST -H "Authorization: Bearer $TOKEN" -d '{"query":"x","text":"重构了登录页"}' .../log
```

路由：`GET /workspaces`、`GET /workspace`、`POST /workspaces`、`POST /open`、`POST /launch`（含起飞前快照）、`POST /log`、`GET /health`。安全说明：仅绑定 127.0.0.1，不出本机；token 是唯一凭证，勿外传。

## MCP 服务器

`hangar mcp` 握手时会**自带使用说明**（instructions 字段）：支持该字段的客户端（Claude Code、Cursor 等）连接成功后自动获得"建完项目要登记、干完活要汇报"的行为习惯，无需手动配置。不支持的客户端请把说明书 MCP 章节的常驻指令模板贴进规则文件。

## MCP 服务器

`hangar mcp` 以 stdio 运行最小 MCP 实现（协议版本 2024-11-05），提供 7 个工具：

| 工具 | 说明 |
|---|---|
| `list_workspaces(query?)` | 列出空间（可过滤） |
| `get_workspace(query)` | 详情：路径/agent/标签/git 状态/启动配置 |
| `open_workspace(query, via?)` | 打开（explorer / terminal） |
| `launch_agent(query, agent?/command?)` | 在空间里启动 agent（新终端窗口） |
| `add_workspace(path, name?, agent?)` | 登记新空间 |
| `upsert_workspace(path, name?, agent?, tags?[])` | 登记或更新：agent 建完新项目后自报家门；已登记则合并标签、更新 agent |
| `log_activity(query, text)` | 往卡片备注追加带时间戳的工作动态（"重构了登录页"） |

配置给 agent：

```bash
claude mcp add hangar -- hangar mcp
```

或 `.mcp.json` / Claude Desktop：

```json
{ "mcpServers": { "hangar": { "command": "hangar", "args": ["mcp"] } } }
```

## 数据说明

- 位置：`%APPDATA%\hangar\hangar.db`（SQLite，WAL 模式）
- 迁移：`PRAGMA user_version` 递增（当前 v2）
- 表：workspaces / groups / tags / workspace_tags / launch_configs / open_events / settings
- 软删除：`workspaces.is_deleted=1`；同路径再登记自动复活并覆盖元数据
- 零遥测零联网；删除软件 = 删掉 `%APPDATA%\hangar` 文件夹即可彻底清理

## 行为细节

- **终端打开链**：统一 `smart_window`——带命令的启动经 `cmd /c start` 开全新控制台的 PowerShell（`-NoExit` 保持窗口）；仅打开目录时 wt 优先。不涉及 Git。
  实测教训三连（都是为了这一条稳）：①git-bash.exe 启动器对句柄/语境极敏感（管道=挂死调用方、NUL/文件=闪退、ShellExecuteW=假启动），弃用；②wt 的命令行把 `;` 当自己的语句分隔符，内嵌 PowerShell 命令会被二次拆分（0x80070002），不能内嵌；③直接 spawn powershell 会继承调用方 stdin，自动化环境下 EOF 导致命令跑完窗口秒退——必须经 `start` 给全新控制台。独立「终端」按钮已移除，命令行需求经「启动…」输入 powershell 满足
- **agent 启动**：统一智能窗口（wt 优先、PowerShell 兜底，不涉及 Git）；启动经 `cmd /c start /wait` 驻留，返回真实进程句柄，运行状态与停止都精确对应窗口
- **进程跟踪**：launch_config_id → Child 的内存注册表；每 2s 轮询；重启后注册表清空（不谎报运行状态）；停止用 `taskkill /T /F` 杀进程树
- **git 状态**：async 命令内 8 线程并行查询，30s 轮询；dirty 含未跟踪文件
- **编辑空间会重建启动配置 id**：运行中编辑会导致该配置的状态跟踪失效（agent 本身不受影响）
