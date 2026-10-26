mod mcp;

use clap::{Parser, Subcommand};
use hangar_core::launcher;
use hangar_core::{CoreError, HangarDb, NewWorkspace};

/// Hangar —— agent 工作空间管理台命令行（与桌面端读写同一份本地数据库）
#[derive(Parser)]
#[command(name = "hangar", version, about)]
struct Cli {
    #[command(subcommand)]
    cmd: Cmd,
}

#[derive(Subcommand)]
enum Cmd {
    /// 列出工作空间（可带过滤词，模糊匹配名称/路径/标签）
    Ls { query: Option<String> },
    /// 登记一个文件夹为工作空间
    Add {
        /// 文件夹路径
        path: String,
        /// 名称（默认取文件夹名）
        #[arg(long)]
        name: Option<String>,
        /// 所属 agent（如 claude-code / codex / cursor）
        #[arg(long)]
        agent: Option<String>,
        /// 标签，可多次使用
        #[arg(long = "tag")]
        tags: Vec<String>,
        /// 分组名（不存在会自动创建）
        #[arg(long)]
        group: Option<String>,
    },
    /// 打开工作空间（资源管理器；--terminal 用 Git Bash 打开）
    Open {
        query: String,
        #[arg(long)]
        terminal: bool,
    },
    /// 在工作空间里运行一条启动配置（agent 起飞）
    Run {
        query: String,
        /// 启动配置名（默认该空间第一条）
        #[arg(long)]
        config: Option<String>,
    },
    /// 收藏 / 取消收藏
    Star { query: String },
    /// 追加标签
    Tag { query: String, tags: Vec<String> },
    /// 移除登记（软删除；绝不删除文件夹本身）
    Rm { query: String },
    /// 作为 MCP 服务器运行（stdio），让 agent 能查询/打开/启动机库里的工作空间
    Mcp,
    /// 启动本地 REST API 服务（127.0.0.1:7333），供脚本/agent 用 HTTP 读写机库
    Serve,
    /// 查看/修改设置（config get [key] / config set key value）
    Config {
        #[arg(value_name = "ACTION")]
        action: String,
        key: Option<String>,
        value: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();
    let db = match HangarDb::open() {
        Ok(d) => d,
        Err(e) => {
            eprintln!("无法打开数据库: {e}");
            std::process::exit(1);
        }
    };
    if let Err(e) = run(&db, cli.cmd) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}

fn run(db: &HangarDb, cmd: Cmd) -> Result<(), String> {
    match cmd {
        Cmd::Ls { query } => {
            let list = db.list().map_err(err)?;
            let q = query.map(|s| s.to_lowercase());
            let mut shown = 0;
            for w in &list {
                if let Some(q) = &q {
                    let hay = format!("{} {} {}", w.name, w.path, w.tags.join(" ")).to_lowercase();
                    if !hay.contains(q) {
                        continue;
                    }
                }
                shown += 1;
                let star = if w.is_favorite { "★" } else { " " };
                let agent = w.agent.as_deref().unwrap_or("-");
                let tags = if w.tags.is_empty() {
                    String::new()
                } else {
                    format!("[{}]", w.tags.join(","))
                };
                println!("{star} {:<24} {:<12} {:<18} {}", w.name, agent, tags, w.path);
            }
            if shown == 0 {
                println!(
                    "（{}）",
                    if q.is_some() { "没有匹配的工作空间" } else { "机库是空的，用 hangar add <路径> 登记" }
                );
            }
            Ok(())
        }
        Cmd::Add { path, name, agent, tags, group } => {
            let p = std::path::Path::new(&path);
            let name = name.unwrap_or_else(|| {
                p.file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| path.clone())
            });
            let group_id = match group.as_deref() {
                Some(g) => Some(db.ensure_group(g, None).map_err(err)?.id),
                None => None,
            };
            let ws = db
                .add(NewWorkspace {
                    name,
                    path,
                    agent,
                    notes: None,
                    icon: None,
                    color: None,
                    group_id,
                    launch_configs: vec![],
                    tags,
                })
                .map_err(err)?;
            println!("已登记: {} → {}", ws.name, ws.path);
            Ok(())
        }
        Cmd::Open { query, terminal } => {
            let w = db.find_by_query(&query).map_err(err)?;
            if terminal {
                launcher::open_terminal_in(&w.path).map_err(io_err)?;
            } else {
                std::process::Command::new("explorer")
                    .arg(&w.path)
                    .spawn()
                    .map_err(io_err)?;
            }
            let _ = db.record_open(&w.id, "cli");
            println!("已打开: {}", w.name);
            Ok(())
        }
        Cmd::Run { query, config } => {
            let w = db.find_by_query(&query).map_err(err)?;
            let lc = config
                .as_deref()
                .and_then(|n| w.launch_configs.iter().find(|c| c.name == n))
                .or_else(|| w.launch_configs.first())
                .ok_or("该空间还没有启动配置（在桌面端编辑空间添加）")?;
            let enabled = db
                .get_settings()
                .ok()
                .and_then(|s| s.get("auto_snapshot").cloned())
                .map(|v| v != "0")
                .unwrap_or(true);
            let _ = hangar_core::snapshot_before_launch(&w.id, &w.path, enabled);
            launcher::spawn_in_shell(&w.path, &lc.command).map_err(io_err)?;
            let _ = db.record_open(&w.id, &format!("launch:{}", lc.name));
            println!("已启动「{}」（{}）: {}", lc.name, lc.command, w.path);
            Ok(())
        }
        Cmd::Star { query } => {
            let w = db.find_by_query(&query).map_err(err)?;
            db.set_favorite(&w.id, !w.is_favorite).map_err(err)?;
            println!("{}: {}", if !w.is_favorite { "已收藏" } else { "已取消收藏" }, w.name);
            Ok(())
        }
        Cmd::Tag { query, tags } => {
            let w = db.find_by_query(&query).map_err(err)?;
            db.add_tags(&w.id, &tags).map_err(err)?;
            println!("已给「{}」打标签: {}", w.name, tags.join(", "));
            Ok(())
        }
        Cmd::Rm { query } => {
            let w = db.find_by_query(&query).map_err(err)?;
            db.soft_delete(&w.id).map_err(err)?;
            println!("已移除登记（文件夹未动）: {}", w.path);
            Ok(())
        }
        Cmd::Mcp => {
            std::process::exit(mcp::run());
        }
        Cmd::Serve => {
            let shared = std::sync::Arc::new(std::sync::Mutex::new(HangarDb::open().map_err(err)?));
            hangar_core::rest::spawn(shared.clone());
            let token = hangar_core::rest::ensure_token(&shared.lock().unwrap());
            println!("REST API 已启动: http://127.0.0.1:{}  (Bearer token: {token})", hangar_core::rest::REST_PORT);
            println!("Ctrl+C 退出");
            loop {
                std::thread::sleep(std::time::Duration::from_secs(3600));
            }
        }
        Cmd::Config { action, key, value } => {
            match (action.as_str(), key, value) {
                ("get", None, _) => {
                    for (k, v) in db.get_settings().map_err(err)? {
                        println!("{k}={v}");
                    }
                    Ok(())
                }
                ("get", Some(k), _) => {
                    let all = db.get_settings().map_err(err)?;
                    println!("{}", all.get(&k).cloned().unwrap_or_default());
                    Ok(())
                }
                ("set", Some(k), Some(v)) => {
                    let mut entries = std::collections::HashMap::new();
                    entries.insert(k, v);
                    db.set_settings(&entries).map_err(err)?;
                    println!("已保存");
                    Ok(())
                }
                _ => Err("用法: hangar config get [key] | hangar config set <key> <value>".into()),
            }
        }
    }
}

fn err(e: CoreError) -> String {
    e.to_string()
}

fn io_err(e: std::io::Error) -> String {
    e.to_string()
}
