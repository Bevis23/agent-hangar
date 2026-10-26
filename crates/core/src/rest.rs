//! 本地 REST API（127.0.0.1:7333）：让任意 AI/脚本用 HTTP 读写机库。
//! 与 MCP/CLI 同源同库。鉴权：`Authorization: Bearer <token>`（token 自动生成，
//! 存 settings.rest_token，设置页可见）；仅 /health 免鉴权。

use crate::git::git_status;
use crate::launcher;
use crate::{HangarDb, NewWorkspace};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub const REST_PORT: u16 = 7333;

type Shared = Arc<Mutex<HangarDb>>;

/// 取（必要时生成）REST token。
pub fn ensure_token(db: &HangarDb) -> String {
    if let Ok(s) = db.get_settings() {
        if let Some(t) = s.get("rest_token") {
            if !t.is_empty() {
                return t.clone();
            }
        }
    }
    let t = uuid::Uuid::now_v7().to_string();
    let mut m = HashMap::new();
    m.insert("rest_token".to_string(), t.clone());
    let _ = db.set_settings(&m);
    t
}

/// 后台起服务（线程内死循环）。端口被占用时静默放弃（不影响主程序）。
pub fn spawn(db: Shared) {
    std::thread::spawn(move || {
        let token = {
            let g = db.lock().unwrap_or_else(|e| e.into_inner());
            ensure_token(&g)
        };
        let server = match tiny_http::Server::http(("127.0.0.1", REST_PORT)) {
            Ok(s) => s,
            Err(_) => return,
        };
        eprintln!("[hangar:rest] listening on 127.0.0.1:{REST_PORT}");
        for mut req in server.incoming_requests() {
            let method = req.method().clone();
            let url = req.url().to_string();
            let (path, query) = match url.split_once('?') {
                Some((p, q)) => (p.to_string(), q.to_string()),
                None => (url.clone(), String::new()),
            };
            let authed = req
                .headers()
                .iter()
                .any(|h| {
                    h.field.equiv("Authorization")
                        && h.value.as_str().trim().ends_with(&token)
                });
            let mut body = String::new();
            let _ = req.as_reader().read_to_string(&mut body);

            let (code, json) = if path == "/health" {
                (200, json!({ "ok": true, "service": "hangar" }).to_string())
            } else if !authed {
                (
                    401,
                    json!({ "error": "未授权：请带 Authorization: Bearer <token>（token 见设置页/ hangar config get rest_token）" })
                        .to_string(),
                )
            } else {
                handle(&db, &method, &path, &query, &body)
            };

            let header =
                tiny_http::Header::from_bytes(b"Content-Type", b"application/json").unwrap();
            let resp = tiny_http::Response::from_string(json)
                .with_status_code(code)
                .with_header(header);
            let _ = req.respond(resp);
        }
    });
}

fn q_param(query: &str, key: &str) -> Option<String> {
    query.split('&').find_map(|kv| {
        let (k, v) = kv.split_once('=')?;
        (k == key).then(|| percent_decode(v))
    })
}

/// 极简 percent-decode（%XX 与 +），够用且零依赖。
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() + 1 && i + 2 <= bytes.len() - 1 || b'%' == bytes[i] && i + 2 < bytes.len() => {
                if i + 2 < bytes.len() {
                    let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                    match u8::from_str_radix(hex, 16) {
                        Ok(b) => {
                            out.push(b);
                            i += 3;
                        }
                        Err(_) => {
                            out.push(bytes[i]);
                            i += 1;
                        }
                    }
                } else {
                    out.push(bytes[i]);
                    i += 1;
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

fn handle(
    db: &Shared,
    method: &tiny_http::Method,
    path: &str,
    query: &str,
    body: &str,
) -> (u16, String) {
    let guard = db.lock().unwrap_or_else(|e| e.into_inner());
    let ok = |v: Value| (200, v.to_string());
    let err = |m: String| (400, json!({ "error": m }).to_string());

    match (method.as_str(), path) {
        ("GET", "/workspaces") => {
            let filter = q_param(query, "query");
            match guard.list() {
                Ok(list) => {
                    let items: Vec<Value> = list
                        .into_iter()
                        .filter(|w| match &filter {
                            Some(f) => format!("{} {} {}", w.name, w.path, w.tags.join(" "))
                                .to_lowercase()
                                .contains(&f.to_lowercase()),
                            None => true,
                        })
                        .map(|w| {
                            json!({ "name": w.name, "path": w.path, "agent": w.agent,
                                    "tags": w.tags, "favorite": w.is_favorite })
                        })
                        .collect();
                    ok(json!({ "count": items.len(), "workspaces": items }))
                }
                Err(e) => err(e.to_string()),
            }
        }
        ("GET", "/workspace") => match q_param(query, "query").ok_or("缺少 query") {
            Ok(q) => match guard.find_by_query(&q) {
                Ok(w) => {
                    let st = git_status(&w.path);
                    ok(json!({ "name": w.name, "path": w.path, "agent": w.agent,
                               "tags": w.tags, "notes": w.notes,
                               "git": { "isRepo": st.is_repo, "branch": st.branch, "dirty": st.dirty },
                               "launchConfigs": w.launch_configs }))
                }
                Err(e) => err(e.to_string()),
            },
            Err(m) => err(m.to_string()),
        },
        ("POST", "/workspaces") => match serde_json::from_str::<Value>(body) {
            Ok(v) => {
                let Some(path) = v.get("path").and_then(Value::as_str).map(str::to_string) else {
                    return err("缺少 path".into());
                };
                let name = v.get("name").and_then(Value::as_str).map(str::to_string);
                let ws = guard.add(NewWorkspace {
                    name: name.unwrap_or_else(|| {
                        std::path::Path::new(&path)
                            .file_name()
                            .map(|n| n.to_string_lossy().to_string())
                            .unwrap_or_else(|| path.clone())
                    }),
                    path,
                    agent: v.get("agent").and_then(Value::as_str).map(str::to_string),
                    notes: None,
                    icon: None,
                    color: None,
                    group_id: None,
                    launch_configs: vec![],
                    tags: vec![],
                });
                match ws {
                    Ok(w) => ok(json!({ "added": w.name, "path": w.path })),
                    Err(e) => err(e.to_string()),
                }
            }
            Err(e) => err(format!("JSON 解析失败: {e}")),
        },
        ("POST", "/open") => match serde_json::from_str::<Value>(body) {
            Ok(v) => {
                let Some(q) = v.get("query").and_then(Value::as_str) else {
                    return err("缺少 query".into());
                };
                match guard.find_by_query(q) {
                    Ok(w) => {
                        let via = v.get("via").and_then(Value::as_str).unwrap_or("explorer");
                        let r = if via == "terminal" {
                            launcher::open_terminal_in(&w.path)
                        } else {
                            std::process::Command::new("explorer")
                                .arg(&w.path)
                                .spawn()
                                .map(|_| ())
                        };
                        match r {
                            Ok(_) => {
                                let _ = guard.record_open(&w.id, "rest");
                                ok(json!({ "opened": w.name }))
                            }
                            Err(e) => err(e.to_string()),
                        }
                    }
                    Err(e) => err(e.to_string()),
                }
            }
            Err(e) => err(format!("JSON 解析失败: {e}")),
        },
        ("POST", "/launch") => match serde_json::from_str::<Value>(body) {
            Ok(v) => {
                let Some(q) = v.get("query").and_then(Value::as_str) else {
                    return err("缺少 query".into());
                };
                match guard.find_by_query(q) {
                    Ok(w) => {
                        let command = v
                            .get("command")
                            .and_then(Value::as_str)
                            .map(str::to_string)
                            .or_else(|| v.get("agent").and_then(Value::as_str).map(str::to_string))
                            .or_else(|| w.launch_configs.first().map(|c| c.command.clone()))
                            .unwrap_or_default();
                        if command.is_empty() {
                            return err("该空间没有启动配置，请传 agent 或 command".into());
                        }
                        let enabled = guard
                            .get_settings()
                            .ok()
                            .and_then(|s| s.get("auto_snapshot").cloned())
                            .map(|x| x != "0")
                            .unwrap_or(true);
                        let _ =
                            crate::snapshot_before_launch(&w.id, &w.path, enabled);
                        match launcher::spawn_in_shell(&w.path, &command) {
                            Ok(_) => {
                                let _ = guard.record_open(&w.id, &format!("launch:{command}"));
                                ok(json!({ "launched": command, "path": w.path }))
                            }
                            Err(e) => err(e.to_string()),
                        }
                    }
                    Err(e) => err(e.to_string()),
                }
            }
            Err(e) => err(format!("JSON 解析失败: {e}")),
        },
        ("POST", "/log") => match serde_json::from_str::<Value>(body) {
            Ok(v) => {
                let (Some(q), Some(text)) = (
                    v.get("query").and_then(Value::as_str),
                    v.get("text").and_then(Value::as_str),
                ) else {
                    return err("缺少 query 或 text".into());
                };
                match guard.log_activity(q, text) {
                    Ok(msg) => ok(json!({ "result": msg })),
                    Err(e) => err(e.to_string()),
                }
            }
            Err(e) => err(format!("JSON 解析失败: {e}")),
        },
        _ => (404, json!({ "error": "未知路由" }).to_string()),
    }
}
