<div align="center">

<video src="docs/media/show.mp4" autoplay loop muted playsinline width="860"></video>

# Hangar

A local workspace manager for AI-assisted development on Windows.

GUI + CLI + MCP + REST, one local database. No account, no network, no telemetry.

[Features](#features) · [Getting started](#getting-started) · [AI integration](#ai-integration) · [Command line](#command-line) · [FAQ](#faq)

</div>

---

## The problem

Coding with AI, folders get out of hand. Claude Code, Codex, and the rest each make their own directory for every task, and after a few weeks your disk fills with `demo`, `test2`, `新建文件夹(3)`. Which ones have uncommitted changes? Which did an agent touch last week? Which are already abandoned?

And it is not just the agents. The projects you started yourself — a tool, a script repo, a folder labeled "to sort" — pile up too, until you no longer remember where they live, which ones are still active, or which can be deleted.

Hangar collects these folders into a single searchable wall of cards, whether an agent made them or you did. Each card tells you where it is, what it is for, which git branch it is on, how many files are uncommitted, and when it was last opened. From the card you can open the folder, or launch an agent directly inside it. Before every launch Hangar takes an automatic snapshot, so a bad edit is one click away from being undone.

## Features

**Collect and organize**

- Import a directory and register every project folder in it, or drag folders in. Agent-made and self-made alike. Folders stay where they are; Hangar only records them.
- Group, tag, favorite, search, and filter by agent or git state.
- Every card shows the current git branch and uncommitted count, refreshed every 30 seconds.

**Launch**

- Start an agent inside the right folder with one click. Hangar detects installed command-line agents automatically; desktop apps can be added manually.
- Select several workspaces and launch them together, each with its own agent.
- Live process status, with a two-step confirm to stop the whole process tree.

**Safety**

- Automatic snapshot before every launch, excluding dependency directories, five kept per workspace. Works even for folders that are not git repositories.
- The exact command is shown for confirmation before anything runs.

**Stay in the app**

- Browse a workspace's files in a side drawer without leaving Hangar.
- View and edit Markdown, text, and images with a built-in viewer; Ctrl+S saves.
- An activity timeline merges launches, git commits, agent reports, and snapshots from the last 14 days.

## AI integration

Hangar exposes the same local database through three interfaces, so the workspaces feed back to the agents that use them:

| Interface | Purpose |
|---|---|
| MCP (7 tools) | `hangar mcp` plugs into any MCP-capable agent. Agents can register a new project, tag and group it, and log what they did. |
| REST | `http://127.0.0.1:7333` with a bearer token, for scripts or programs that prefer HTTP. |
| CLI | `hangar ls / open / run / add / star / tag / rm / config / mcp / serve`. |

A concrete flow: tell your agent to create a new project. When it is done it calls `upsert_workspace` to register the folder with tags and a group, then logs a line with `log_activity`. Switch back to Hangar and the timeline already shows the work.

See [Connecting MCP](docs/MCP接入指南.md) for step-by-step setup instructions for Claude Code, Codex, and other MCP-capable agents.

## Getting started

| Package | Size | Notes |
|---|---|---|
| `Hangar_0.3.0_x64-setup.exe` | 2.4 MB | Installer, no admin rights required |
| `hangar-0.3.0-portable-win-x64.zip` | 4.4 MB | Portable, includes the CLI |

Requires Windows 10 or 11. To build from source: `npm install && npm run tauri dev` (Node 20+, Rust MSVC).

Sixty-second start: import your project directory, click "Launch…" on a card, pick an agent, confirm, and it is running in the right directory.

## Command line

```bash
hangar ls                      # list (fuzzy filter)
hangar open demo --terminal    # open in a terminal
hangar run demo                # run its launch config (auto-snapshot first)
hangar add D:\work\x --agent claude-code --tag learning
hangar mcp                     # run as an MCP server
hangar serve                   # start the REST API
```

The CLI shares the same SQLite database (WAL) as the GUI, so both can be used at once.

## FAQ

**Does it touch my files?** No. Everything is read-only; removing a workspace only deletes its registry entry.

**Where is my data?** One local SQLite file at `%APPDATA%\hangar`. No network, no telemetry.

**Does it work without Git?** Yes. Terminal selection falls back to Windows Terminal or PowerShell, and every feature works regardless.

**Why does launching a desktop agent do nothing?** Single-instance apps just focus their existing window. That is expected behavior.

## Documentation

- [中文说明书（完整版）](docs/说明书.md)
- [技术指南](docs/GUIDE.zh-CN.md)
- [MCP 接入指南](docs/MCP接入指南.md)
- [中文 README](README.zh-CN.md)

## License

[MIT](LICENSE)
