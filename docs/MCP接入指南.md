# Hangar MCP 接入指南

Hangar 通过 MCP（Model Context Protocol）把机库里的工作空间开放给你的 AI agent。接入后，agent 可以直接查询、打开、启动你的项目，并在干活的过程中自动登记新项目、汇报进展。整个过程走本地 stdio，不联网、不经过任何第三方。

## 它能做什么

接入后，agent 会多出 7 个工具：

| 工具 | 作用 | 典型场景 |
|---|---|---|
| `list_workspaces` | 列出机库里登记的工作空间，可按名称/路径/标签过滤 | 「帮我看看现在有哪些项目」 |
| `get_workspace` | 查看某个空间的详情：路径、agent、标签、git 分支、未提交数、启动配置 | 「博客项目现在在哪个分支、改了多少」 |
| `open_workspace` | 用资源管理器或终端打开某个空间 | 「打开 demo 的文件夹」 |
| `launch_agent` | 在该空间里新开终端并运行一个 agent / 命令 | 「在 demo 里起一个 codex」 |
| `add_workspace` | 把本机文件夹登记进机库 | 「把这个文件夹收进机库」 |
| `upsert_workspace` | 登记或更新一个空间（已存在则合并标签、更新 agent） | agent 建完项目后主动登记 |
| `log_activity` | 给某个空间追加一条带时间戳的工作动态 | agent 干完活汇报一句 |

## 前置条件

1. 已经装好 Hangar（安装版或便携版均可）。
2. 找到 `hangar` 可执行文件的路径。不同安装方式的路径不同：
   - 便携版解压后，就是解压目录里的 `hangar-cli.exe`。
   - 安装版装完后，`hangar-cli.exe` 位于安装目录（或已加入系统 PATH，可直接用 `hangar`）。
3. 确认命令行能跑通：在终端里执行 `hangar mcp --help` 或直接 `hangar ls`，能正常输出即代表 CLI 可用。

下面的配置统一以「CLI 已在 PATH 中」为前提，命令写 `hangar mcp`。如果你的 CLI 不在 PATH，把 `hangar` 换成 `hangar-cli.exe` 的完整路径。

## 接入 Claude Code

Claude Code 通过 `claude mcp add` 注册 MCP 服务器，配置写入 `~/.claude.json`。

在终端里执行：

```bash
claude mcp add hangar -- cmd /c hangar mcp
```

说明：Hangar 的 MCP 是 stdio 协议，Windows 上需要 `cmd /c` 包一层。注册后重启 Claude Code（或新建会话）即可生效。

也可以直接编辑 `~/.claude.json`，在 `mcpServers` 里加一段：

```json
{
  "mcpServers": {
    "hangar": {
      "type": "stdio",
      "command": "cmd",
      "args": ["/c", "hangar", "mcp"]
    }
  }
}
```

## 接入 Codex

Codex 使用 `codex mcp add`，配置写入 `~/.codex/config.toml`。

```bash
codex mcp add hangar -- cmd /c hangar mcp
```

对应的 `config.toml` 配置：

```toml
[mcp_servers.hangar]
command = "cmd"
args = ["/c", "hangar", "mcp"]
```

## 接入其他 agent（通用做法）

绝大多数支持 MCP 的 agent 都能读标准 JSON 配置，格式与上面 Claude Code 的 `mcpServers` 一致。核心就三条：

```json
{
  "mcpServers": {
    "hangar": {
      "command": "cmd",
      "args": ["/c", "hangar", "mcp"]
    }
  }
}
```

- 有的 agent 用 `.mcp.json`（放在项目根目录），有的用全局配置文件、有的用界面里的「添加 MCP 服务器」入口。字段名一般是 `command` + `args`（stdio 类型）。
- 如果它要求直接填「可执行文件 + 参数」两个框，就填：可执行文件 `cmd`，参数 `/c hangar mcp`。
- 具体以你用的 agent 的 MCP 文档为准，把上面三条字段对过去即可。

## 验证是否接入成功

接入后，在 agent 里发一句：

> 用 list_workspaces 看看我机库里有哪些项目。

如果它返回了机库里的项目列表，就说明通了。也可以先手动测一下协议本身是否正常——在终端跑下面这条，能看到返回的 JSON 即代表服务端没问题：

```bash
printf '{"jsonrpc":"2.0","id":1,"method":"tools/list"}\n' | hangar mcp
```

## 一个端到端例子

对 agent 说：

> 在 D:\work 里建一个「记账本」项目，写完基础架子后登记进 Hangar 机库，归到「实验」分组，打上「demo」标签，并记一句你做了什么。

agent 会依次调用：

1. 建项目文件夹、写代码。
2. `upsert_workspace`，传 `path=D:\work\记账本`、`group=实验`、`tags=["demo"]`、`agent=<它自己的名字>`。
3. `log_activity`，传一句「搭好了记账本的基础架子」。

切回 Hangar 桌面端，卡片墙里已经多出「记账本」，分组、标签、动态都齐了。

## 常见问题

**为什么是 `cmd /c`？** Hangar 的 MCP 走 stdio（标准输入输出）。Windows 上直接启动 exe 在部分 agent 里会有控制台继承问题，`cmd /c` 包一层最稳。

**agent 说连不上 / 找不到命令？** 大概率是 `hangar` 不在 PATH。把命令里的 `hangar` 换成 `hangar-cli.exe` 的完整路径，例如 `C:\hangar\hangar-cli.exe`。

**会和桌面端冲突吗？** 不会。MCP、CLI、桌面端共用同一个 SQLite 数据库（WAL 模式），可以同时读写。

**agent 会不会乱动我的文件？** MCP 只暴露了「查询、打开、启动、登记、记动态」这几类能力，没有删除、没有写文件。真正改文件的是 agent 自己在文件夹里做的事，与 Hangar 无关。
