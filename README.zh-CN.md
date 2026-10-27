<div align="center">

<video src="docs/media/show.mp4" autoplay loop muted playsinline width="860"></video>

# Hangar（机库）

Windows 上管理 AI 工作空间的本地工具。

GUI + CLI + MCP + REST，同一份本地数据。零联网、零账号、零遥测。

[核心功能](#核心功能) · [快速开始](#快速开始) · [AI 集成](#ai-集成) · [命令行](#命令行) · [FAQ](#faq)

</div>

---

## 它解决什么问题

AI 编程助手每次干活都会新建文件夹。Claude Code、Codex、豆包、WorkBuddy，各建各的，几个星期下来硬盘里就堆满了 `demo`、`test2`、`新建文件夹(3)`。哪些有没提交的代码？哪些上周被 AI 动过？哪些早就烂尾了？

Hangar 把这些文件夹收进一屏可搜索的卡片墙。每张卡片都写着：它属于哪个 AI、当前在哪个 git 分支、有几个文件没提交、上次打开是什么时候。从卡片可以打开文件夹，也可以直接在文件夹里启动一个 AI。每次启动前 Hangar 自动快照，改坏了点一下就回滚。

## 核心功能

**整理**

- 导入一个目录，把里面的项目文件夹全部登记进来，也可以直接拖拽。文件夹原地不动，Hangar 只做记录。
- 分组、标签、收藏、搜索，按 agent 或 git 状态筛选。
- 每张卡片实时显示 git 分支和未提交数，每 30 秒刷新。

**启动**

- 一键在正确的文件夹里启动 AI。命令行形态的 agent 自动检测，桌面应用手动添加。
- 多选几个空间一起启动，各用各的 agent。
- 运行状态实时可见，两段式确认停止整棵进程树。

**安全网**

- 每次启动前自动快照，排除依赖目录，每个空间保留 5 份。不是 git 仓库的文件夹同样有效。
- 任何命令执行前完整展示，确认后才运行。

**应用内闭环**

- 在侧边抽屉里浏览工作空间文件，不用离开 Hangar。
- 内置查看器查看和编辑 Markdown、文本、图片，Ctrl+S 保存。
- 动态时间线汇总最近 14 天的启动、git 提交、agent 汇报和快照。

## AI 集成

Hangar 用三个接口暴露同一份本地数据，让工作空间反过来服务于使用它们的 AI：

| 接口 | 用途 |
|---|---|
| MCP（7 个工具） | `hangar mcp` 接入任意支持 MCP 的 agent。agent 可以登记新项目、打标签分组、记录干了什么。 |
| REST | `http://127.0.0.1:7333`，带 Bearer token，给偏好 HTTP 的脚本或程序用。 |
| CLI | `hangar ls / open / run / add / star / tag / rm / config / mcp / serve`。 |

一个具体的流程：让 agent 新建一个项目，它完成后调用 `upsert_workspace` 登记文件夹、打上标签归入分组，再用 `log_activity` 记一笔。切回 Hangar，时间线里已经躺着这段工作记录。

接入步骤见 [MCP 接入指南](docs/MCP接入指南.md)，含 Claude Code、Codex 及其他 agent 的配置方法。

## 快速开始

| 产物 | 体积 | 说明 |
|---|---|---|
| `Hangar_0.3.0_x64-setup.exe` | 2.4 MB | 安装版，无需管理员权限 |
| `hangar-0.3.0-portable-win-x64.zip` | 4.4 MB | 便携版，含 CLI |

要求 Windows 10/11。从源码构建：`npm install && npm run tauri dev`（Node 20+、Rust MSVC）。

六十秒上手：导入项目目录，点卡片上的「启动…」，选一个 agent，确认，它就已经在正确的目录里跑起来了。

## 命令行

```bash
hangar ls                      # 列出（模糊过滤）
hangar open demo --terminal    # 在终端打开
hangar run demo                # 运行其启动配置（先自动快照）
hangar add D:\work\x --agent claude-code --tag learning
hangar mcp                     # 作为 MCP 服务器运行
hangar serve                   # 启动 REST API
```

CLI 与桌面端共用同一个 SQLite（WAL），可以同时使用。

## FAQ

**会动我的文件吗？** 不会。全程只读，移除工作空间只删除登记记录。

**数据在哪？** 本机一个 SQLite 文件，位于 `%APPDATA%\hangar`。零联网、零遥测。

**没装 Git 能用吗？** 能。终端自动回退 Windows Terminal 或 PowerShell，所有功能不受影响。

**桌面 AI 点启动没反应？** 单实例应用只会聚焦已有窗口，这是正常行为。

## 文档

- [使用说明书（完整版）](docs/说明书.md)
- [技术指南](docs/GUIDE.zh-CN.md)
- [MCP 接入指南](docs/MCP接入指南.md)
- [English README](README.md)

## License

[MIT](LICENSE)
