//! `hangar mcp`：作为 MCP（Model Context Protocol）服务器在 stdio 上运行。
//! 让任意支持 MCP 的 agent（Claude Code / Codex / …）能查询机库、打开空间、
//! 甚至替你启动另一个 agent。与桌面端共用同一数据库（WAL，可并发）。
//!
//! 协议实现是手写的最小子集：initialize / tools/list / tools/call / ping。

use hangar_core::launcher;
use hangar_core::{git_status, HangarDb, NewLaunchConfig, NewWorkspace, WorkspaceUpdate};
use serde_json::{json, Value};
use std::io::{BufRead, Write};

pub fn run() -> i32 {
    let db = match HangarDb::open() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("hangar mcp: 无法打开数据库: {e}");
            return 1;
        }
    };

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(line) else { continue };
        // 通知（无 id）不回复
        let Some(id) = msg.get("id").cloned() else { continue };
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or_else(|| json!({}));

        let response = match method {
            "initialize" => wrap(
                id,
                json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": { "tools": {} },
                    "serverInfo": { "name": "hangar", "version": env!("CARGO_PKG_VERSION") },
                    "instructions": "你已接入 Hangar 机库——主人电脑上所有 AI 工作空间的登记处。规则：1. 你或主人在本机新建了项目文件夹后，立即调用 upsert_workspace 登记（path=项目路径，agent=你的名字，按内容打 1-2 个 tags，group=合适的分组名，分组不存在会自动创建）。2. 每完成一项工作，调用 log_activity 用一句话汇报（做了什么）。3. 主人让你查/打开/启动某个空间时，用 list_workspaces / get_workspace / open_workspace / launch_agent 直接完成，不要让主人手动操作。工具返回的路径可直接用于后续调用。"
                }),
            ),
            "ping" => wrap(id, json!({})),
            "tools/list" => wrap(id, json!({ "tools": tool_defs() })),
            "tools/call" => wrap(id, call_tool(&db, &params)),
            _ => json!({
                "jsonrpc": "2.0",
                "id": id,
                "error": { "code": -32601, "message": format!("unknown method: {method}") }
            }),
        };
        let _ = writeln!(out, "{response}");
        let _ = out.flush();
    }
    0
}

fn wrap(id: Value, result: Value) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "result": result })
}

fn text_result(text: String) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": false })
}

fn error_result(text: String) -> Value {
    json!({ "content": [{ "type": "text", "text": text }], "isError": true })
}

fn tool_defs() -> Value {
    json!([
        {
            "name": "list_workspaces",
            "description": "列出 Hangar 机库里登记的工作空间（agent 的工作文件夹）。可按关键词过滤名称/路径/标签。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "可选，模糊匹配名称/路径/标签" }
                }
            }
        },
        {
            "name": "get_workspace",
            "description": "查看某个工作空间的详情：路径、所属 agent、标签、git 分支与未提交数、启动配置。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "名称或路径的模糊匹配" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "open_workspace",
            "description": "打开某个工作空间：资源管理器显示，或在终端中打开（默认 Git Bash）。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "via": { "type": "string", "enum": ["explorer", "terminal"], "description": "默认 explorer" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "launch_agent",
            "description": "在某个工作空间里启动一个编码 agent（新终端窗口，cd 到该目录后运行命令）。不传 agent/command 时使用该空间第一条启动配置。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string" },
                    "agent": { "type": "string", "description": "agent 命令，如 claude / codex / gemini" },
                    "command": { "type": "string", "description": "自定义命令（优先于 agent）" }
                },
                "required": ["query"]
            }
        },
        {
            "name": "add_workspace",
            "description": "把一个本机文件夹登记进 Hangar 机库。支持标签与分组（分组不存在会自动创建）。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string" },
                    "name": { "type": "string" },
                    "agent": { "type": "string", "description": "如 claude-code / codex" },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "group": { "type": "string", "description": "分组名，不存在会自动创建" }
                },
                "required": ["path"]
            }
        },
        {
            "name": "upsert_workspace",
            "description": "登记或更新一个工作空间。agent 在新文件夹里建完项目后应主动调用此工具，把项目收进机库（已登记则合并标签并更新 agent）。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "项目文件夹绝对路径" },
                    "name": { "type": "string" },
                    "agent": { "type": "string", "description": "你的名字，如 zcode / claude-code" },
                    "tags": { "type": "array", "items": { "type": "string" } },
                    "group": { "type": "string", "description": "分组名，不存在会自动创建" }
                },
                "required": ["path"]
            }
        },
        {
            "name": "log_activity",
            "description": "向某个工作空间追加一条带时间戳的工作动态（会显示在 Hangar 卡片的备注里）。完成一项工作后调用，方便主人回顾每个空间里干过什么。",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "工作空间名称/路径模糊匹配" },
                    "text": { "type": "string", "description": "做了什么，一句话，如：重构了登录页" }
                },
                "required": ["query", "text"]
            }
        }
    ])
}

fn call_tool(db: &HangarDb, params: &Value) -> Value {
    let name = params.get("name").and_then(Value::as_str).unwrap_or("");
    let args = params.get("arguments").cloned().unwrap_or_else(|| json!({}));
    let q = |k: &str| args.get(k).and_then(Value::as_str).map(str::to_string);
    let arr = |k: &str| -> Vec<String> {
        args.get(k)
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
            .unwrap_or_default()
    };

    let result: Result<String, String> = match name {
        "list_workspaces" => (|| {
            let list = db.list().map_err(e)?;
            let groups: std::collections::HashMap<String, String> = db
                .list_groups()
                .unwrap_or_default()
                .into_iter()
                .map(|g| (g.id, g.name))
                .collect();
            let filter = q("query").map(|s| s.to_lowercase());
            let mut lines = Vec::new();
            for w in &list {
                if let Some(f) = &filter {
                    let hay = format!("{} {} {}", w.name, w.path, w.tags.join(" ")).to_lowercase();
                    if !hay.contains(f) {
                        continue;
                    }
                }
                let star = if w.is_favorite { "★" } else { " " };
                let group = w
                    .group_id
                    .as_deref()
                    .and_then(|id| groups.get(id))
                    .cloned()
                    .unwrap_or_else(|| "-".into());
                let st = git_status(&w.path);
                let git = if st.is_repo {
                    format!(
                        "{}{}",
                        st.branch.unwrap_or_else(|| "?".into()),
                        if st.dirty > 0 { format!(" ~{}", st.dirty) } else { String::new() }
                    )
                } else {
                    "-".into()
                };
                lines.push(format!(
                    "{star} {} | 组:{} | {} | 标签:{} | git:{} | {}",
                    w.name,
                    group,
                    w.agent.as_deref().unwrap_or("-"),
                    if w.tags.is_empty() { "-".to_string() } else { w.tags.join(",") },
                    git,
                    w.path
                ));
            }
            if lines.is_empty() {
                Ok("机库为空或没有匹配".to_string())
            } else {
                Ok(format!("共 {} 个：\n{}", lines.len(), lines.join("\n")))
            }
        })(),
        "get_workspace" => (|| {
            let query = q("query").ok_or("缺少 query")?;
            let w = db.find_by_query(&query).map_err(e)?;
            let st = git_status(&w.path);
            let cfgs = if w.launch_configs.is_empty() {
                "-".to_string()
            } else {
                w.launch_configs
                    .iter()
                    .map(|c| format!("{}: {} ({})", c.name, c.command, c.shell))
                    .collect::<Vec<_>>()
                    .join("; ")
            };
            let last = w
                .last_opened_at
                .map(fmt_epoch)
                .unwrap_or_else(|| "从未打开".into());
            Ok(format!(
                "名称: {}\n路径: {}\nagent: {}\n标签: {}\ngit: {}\n最近打开: {}\n备注/动态: {}\n启动配置: {}",
                w.name,
                w.path,
                w.agent.as_deref().unwrap_or("-"),
                if w.tags.is_empty() { "-".to_string() } else { w.tags.join(", ") },
                if st.is_repo {
                    format!(
                        "分支 {}，{}",
                        st.branch.unwrap_or_else(|| "?".into()),
                        if st.dirty > 0 { format!("{} 个文件未提交", st.dirty) } else { "干净".to_string() }
                    )
                } else {
                    "非 git 目录".to_string()
                },
                last,
                w.notes.as_deref().unwrap_or("-"),
                cfgs
            ))
        })(),
        "open_workspace" => (|| {
            let query = q("query").ok_or("缺少 query")?;
            let w = db.find_by_query(&query).map_err(e)?;
            match q("via").as_deref() {
                Some("terminal") => {
                    launcher::open_terminal_in(&w.path).map_err(io)?;
                }
                _ => {
                    std::process::Command::new("explorer")
                        .arg(&w.path)
                        .spawn()
                        .map_err(io)?;
                }
            }
            let _ = db.record_open(&w.id, "mcp");
            Ok(format!("已打开: {}（{}）", w.name, w.path))
        })(),
        "launch_agent" => (|| {
            let query = q("query").ok_or("缺少 query")?;
            let w = db.find_by_query(&query).map_err(e)?;
            let command = q("command")
                .or_else(|| q("agent"))
                .or_else(|| w.launch_configs.first().map(|c| c.command.clone()))
                .ok_or("该空间没有启动配置，请传 agent 或 command")?;
            let enabled = db
                .get_settings()
                .ok()
                .and_then(|s| s.get("auto_snapshot").cloned())
                .map(|v| v != "0")
                .unwrap_or(true);
            let _ = hangar_core::snapshot_before_launch(&w.id, &w.path, enabled);
            launcher::spawn_in_shell(&w.path, &command).map_err(io)?;
            let _ = db.record_open(&w.id, &format!("launch:{}", command));
            Ok(format!("已在新终端启动 `{command}`（目录 {}）", w.path))
        })(),
        "add_workspace" => (|| {
            let path = q("path").ok_or("缺少 path")?;
            let name = q("name").unwrap_or_else(|| {
                std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone())
            });
            let group = q("group");
            let group_id = match group.as_deref() {
                Some(g) => Some(db.ensure_group(g, None).map_err(e)?.id),
                None => None,
            };
            let ws = db
                .add(NewWorkspace {
                    name,
                    path,
                    agent: q("agent"),
                    notes: None,
                    icon: None,
                    color: None,
                    group_id,
                    launch_configs: vec![],
                    tags: arr("tags"),
                })
                .map_err(e)?;
            Ok(format!("已登记: {} → {}", ws.name, ws.path))
        })(),
        "upsert_workspace" => (|| {
            let path = q("path").ok_or("缺少 path")?;
            let name = q("name").unwrap_or_else(|| {
                std::path::Path::new(&path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone())
            });
            let agent = q("agent");
            let group = q("group");
            let tags = arr("tags");
            match db.add(NewWorkspace {
                name,
                path: path.clone(),
                agent: agent.clone(),
                notes: None,
                icon: None,
                color: None,
                group_id: None,
                launch_configs: vec![],
                tags: tags.clone(),
            }) {
                Ok(ws) => {
                    let mut extra = String::new();
                    if let Some(gname) = group.as_deref() {
                        let g = db.ensure_group(gname, None).map_err(e)?;
                        db.set_workspace_group(&ws.id, Some(&g.id)).map_err(e)?;
                        extra = format!("，归入分组「{}」", g.name);
                    }
                    Ok(format!("已登记进机库: {} → {}{}", ws.name, ws.path, extra))
                }
                Err(hangar_core::CoreError::AlreadyRegistered) => {
                    // 已登记：规范化路径后找到旧记录，合并标签、更新 agent
                    let norm = normalize(&path)?;
                    let list = db.list().map_err(e)?;
                    let w = list
                        .iter()
                        .find(|w| w.path.eq_ignore_ascii_case(&norm))
                        .ok_or("该路径已登记但未能定位记录")?;
                    db.add_tags(&w.id, &tags).map_err(e)?;
                    let updated_agent = match agent.as_deref() {
                        Some(a) => {
                            db.set_workspace_agent(&w.id, Some(a)).map_err(e)?;
                            true
                        }
                        None => false,
                    };
                    let mut extra = String::new();
                    if let Some(gname) = group.as_deref() {
                        let g = db.ensure_group(gname, None).map_err(e)?;
                        db.set_workspace_group(&w.id, Some(&g.id)).map_err(e)?;
                        extra = format!("，归入分组「{}」", g.name);
                    }
                    Ok(format!(
                        "已更新「{}」：合并 {} 个标签{}{}",
                        w.name,
                        tags.len(),
                        if updated_agent { "，更新 agent" } else { "" },
                        extra
                    ))
                }
                Err(other) => Err(e(other)),
            }
        })(),
        "log_activity" => (|| {
            let query = q("query").ok_or("缺少 query")?;
            let text = q("text").ok_or("缺少 text")?;
            let w = db.find_by_query(&query).map_err(e)?;
            let old = w.notes.as_deref().unwrap_or("").trim_end();
            let notes = format!("{old}\n[{}] {}", now_stamp(), text.trim());
            db.update(WorkspaceUpdate {
                id: w.id.clone(),
                name: w.name.clone(),
                agent: w.agent.clone(),
                notes: Some(notes),
                icon: w.icon.clone(),
                color: w.color.clone(),
                group_id: w.group_id.clone(),
                launch_configs: w
                    .launch_configs
                    .iter()
                    .map(|c| NewLaunchConfig {
                        name: c.name.clone(),
                        command: c.command.clone(),
                        shell: c.shell.clone(),
                        confirm_skip: c.confirm_skip,
                    })
                    .collect(),
                tags: w.tags.clone(),
            })
            .map_err(e)?;
            Ok(format!("已记录动态到「{}」：{}", w.name, text.trim()))
        })(),
        _ => Err(format!("unknown tool: {name}")),
    };

    match result {
        Ok(text) => text_result(text),
        Err(text) => error_result(text),
    }
}

fn e(err: hangar_core::CoreError) -> String {
    err.to_string()
}

fn io(err: std::io::Error) -> String {
    err.to_string()
}

/// 规范化路径（canonicalize + 去 \\?\ 前缀），路径不存在时报错。
fn normalize(p: &str) -> Result<String, String> {
    let c = std::fs::canonicalize(p).map_err(|_| format!("路径不存在: {p}"))?;
    let mut s = c.to_string_lossy().to_string();
    if let Some(st) = s.strip_prefix(r"\\?\") {
        s = st.to_string();
    }
    Ok(s)
}

/// epoch 秒 → "YYYY-MM-DD HH:MM"（UTC+8），供 log_activity 与详情展示复用。
fn fmt_epoch(secs: i64) -> String {
    let secs = secs + 8 * 3600;
    let days = secs.div_euclid(86400);
    let rem = secs.rem_euclid(86400);
    let (y, m, d) = civil_from_days(days);
    format!(
        "{y:04}-{m:02}-{d:02} {:02}:{:02}",
        rem / 3600,
        (rem % 3600) / 60
    )
}

/// 当前时间戳（本地时区按 UTC+8 计算）：YYYY-MM-DD HH:MM
fn now_stamp() -> String {
    fmt_epoch(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs() as i64,
    )
}

/// Howard Hinnant 的 civil_from_days 算法
fn civil_from_days(z: i64) -> (i64, u64, u64) {
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}
