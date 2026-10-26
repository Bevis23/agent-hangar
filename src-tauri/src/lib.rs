use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::{Child, Command};
use std::sync::{Mutex, MutexGuard};

use hangar_core::git::{self, GitStatus};
use hangar_core::launcher;
use hangar_core::{
    activity_feed as build_feed, CandidateDir, FailedImport, FsEntry, Group, HangarDb, ImportSummary,
    LaunchConfig, NewGroup, NewWorkspace, QuickLaunch, SnapInfo, TextFile, Workspace,
    WorkspaceUpdate,
};
use serde::Serialize;
use tauri::{
    menu::{Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager, State,
};

type DbState = Mutex<HangarDb>;
/// 运行中的启动进程注册表：launch_config_id → 终端子进程。
/// Hangar 重启后表为空，对应状态显示"未知"。
type ProcessesState = Mutex<HashMap<String, Child>>;
/// 托盘图标句柄（重建菜单时用）
type TrayState = Mutex<Option<tauri::tray::TrayIcon<tauri::Wry>>>;

fn db<'a>(state: &'a State<'_, DbState>) -> MutexGuard<'a, HangarDb> {
    state.lock().expect("hangar database mutex poisoned")
}

#[tauri::command]
fn list_workspaces(state: State<DbState>) -> Result<Vec<Workspace>, String> {
    db(&state).list().map_err(|e| e.to_string())
}

#[tauri::command]
fn add_workspace(state: State<DbState>, payload: NewWorkspace) -> Result<Workspace, String> {
    db(&state).add(payload).map_err(|e| e.to_string())
}

#[tauri::command]
fn update_workspace(state: State<DbState>, payload: WorkspaceUpdate) -> Result<Workspace, String> {
    db(&state).update(payload).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_favorite(state: State<DbState>, id: String, favorite: bool) -> Result<(), String> {
    db(&state).set_favorite(&id, favorite).map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_workspace(state: State<DbState>, id: String) -> Result<(), String> {
    db(&state).soft_delete(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn record_open(state: State<DbState>, id: String, via: String) -> Result<(), String> {
    db(&state).record_open(&id, &via).map_err(|e| e.to_string())
}

#[tauri::command]
fn list_groups(state: State<DbState>) -> Result<Vec<Group>, String> {
    db(&state).list_groups().map_err(|e| e.to_string())
}

#[tauri::command]
fn create_group(state: State<DbState>, payload: NewGroup) -> Result<Group, String> {
    db(&state)
        .create_group(&payload.name, payload.icon.as_deref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
fn delete_group(state: State<DbState>, id: String) -> Result<(), String> {
    db(&state).delete_group(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn set_workspace_group(state: State<DbState>, id: String, group: Option<String>) -> Result<(), String> {
    db(&state)
        .set_workspace_group(&id, group.as_deref())
        .map_err(|e| e.to_string())
}

/// 扫描指定根目录下的候选项目文件夹（深度 ≤ 2，按 marker 识别，排除已登记）。
#[tauri::command]
fn scan_import_root(state: State<DbState>, path: String, all: bool) -> Result<Vec<CandidateDir>, String> {
    let handle = db(&state);
    let registered: HashSet<String> = handle
        .all_paths()
        .map_err(|e| e.to_string())?
        .into_iter()
        .collect();
    hangar_core::scan(&path, &registered, all).map_err(|e| e.to_string())
}

/// 批量登记工作空间：逐个入库，单个失败不影响其余。
#[tauri::command]
fn import_workspaces(
    state: State<DbState>,
    payloads: Vec<NewWorkspace>,
) -> Result<ImportSummary, String> {
    let handle = db(&state);
    let mut added = Vec::new();
    let mut failed = Vec::new();
    for payload in payloads {
        let path = payload.path.clone();
        match handle.add(payload) {
            Ok(ws) => added.push(ws),
            Err(e) => failed.push(FailedImport {
                path,
                reason: e.to_string(),
            }),
        }
    }
    Ok(ImportSummary { added, failed })
}

/// 在资源管理器中显示该文件夹。只读操作，符合"文件夹原地不动"承诺。
#[tauri::command]
fn open_in_explorer(path: String) -> Result<(), String> {
    Command::new("explorer")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("打开资源管理器失败: {e}"))?;
    Ok(())
}

/// 在终端中打开该目录（Windows Terminal 优先，PowerShell 兜底）。
#[tauri::command]
fn open_in_terminal(path: String) -> Result<(), String> {
    launcher::open_terminal_in(&path).map_err(|e| format!("打开终端失败: {e}"))
}

#[tauri::command]
fn get_settings(state: State<DbState>) -> Result<HashMap<String, String>, String> {
    db(&state).get_settings().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_settings(state: State<DbState>, entries: HashMap<String, String>) -> Result<(), String> {
    db(&state).set_settings(&entries).map_err(|e| e.to_string())
}

/// 用用户配置的编辑器命令打开该目录（模板中 {path} 会被替换）。
#[tauri::command]
fn open_in_editor(state: State<DbState>, path: String) -> Result<(), String> {
    let template = db(&state)
        .get_settings()
        .map_err(|e| e.to_string())?
        .get("editor_cmd")
        .cloned()
        .filter(|s| !s.trim().is_empty())
        .ok_or("尚未配置编辑器命令：右上角「设置」→ 终端与编辑器")?;
    let mut parts = template.split_whitespace();
    let prog = parts.next().ok_or("编辑器命令为空")?;
    let args: Vec<String> = parts
        .map(|t| if t == "{path}" { path.clone() } else { t.to_string() })
        .collect();
    Command::new(prog)
        .args(&args)
        .spawn()
        .map_err(|e| format!("启动编辑器失败: {e}（检查命令是否已安装）"))?;
    Ok(())
}

/// 应用内文件浏览：列出某目录内容（只读）。
#[tauri::command]
fn list_dir(path: String) -> Result<Vec<FsEntry>, String> {
    hangar_core::list_dir(&path).map_err(|e| e.to_string())
}

/// 用系统默认程序打开文件 / 资源管理器显示文件夹。
#[tauri::command]
fn open_entry(path: String) -> Result<(), String> {
    hangar_core::open_entry(&path).map_err(|e| e.to_string())
}

/// 读取文本文件（内置查看器用；2MB 上限截断）。
#[tauri::command]
fn read_text_file(path: String) -> Result<TextFile, String> {
    hangar_core::read_text_file(&path).map_err(|e| e.to_string())
}

/// 保存文本文件（内置编辑器用）。
#[tauri::command]
fn write_text_file(path: String, content: String) -> Result<(), String> {
    hangar_core::write_text_file(&path, &content).map_err(|e| e.to_string())
}

/// 某空间的快照列表（新→旧）。
#[tauri::command]
fn list_snapshots(workspace_id: String) -> Result<Vec<SnapInfo>, String> {
    hangar_core::list_snapshots(&workspace_id).map_err(|e| e.to_string())
}

/// 恢复快照（覆盖解压回工作空间），返回恢复的文件数。
#[tauri::command]
fn restore_snapshot(workspace_id: String, file: String, path: String) -> Result<u64, String> {
    hangar_core::restore_snapshot(&workspace_id, &file, &path).map_err(|e| e.to_string())
}

/// 删除一份快照。
#[tauri::command]
fn delete_snapshot(workspace_id: String, file: String) -> Result<(), String> {
    hangar_core::delete_snapshot(&workspace_id, &file).map_err(|e| e.to_string())
}

/// 活动时间线（起飞/打开/工作动态/git 提交聚合）。
#[tauri::command]
fn activity_feed(days: u32, state: State<DbState>) -> Result<Vec<hangar_core::FeedItem>, String> {
    build_feed(&db(&state), days).map_err(|e| e.to_string())
}

/// 并行查询一批目录的 git 状态（分支 + 未提交数）。
/// 在阻塞线程池中跑、内部再开 8 个线程分片，不阻塞 UI。
#[tauri::command]
async fn git_statuses(paths: Vec<String>) -> HashMap<String, GitStatus> {
    tauri::async_runtime::spawn_blocking(move || {
        let out = Mutex::new(HashMap::new());
        let threads = 8;
        let chunk_size = paths.len().div_ceil(threads).max(1);
        std::thread::scope(|s| {
            for part in paths.chunks(chunk_size) {
                let out = &out;
                s.spawn(move || {
                    for p in part {
                        let st = git::git_status(p);
                        out.lock().expect("git statuses map poisoned").insert(p.clone(), st);
                    }
                });
            }
        });
        out.into_inner().expect("git statuses map poisoned")
    })
    .await
    .unwrap_or_default()
}

/// 设置里的"起飞前自动快照"开关（缺省视为开）。
fn snapshot_enabled(state: &State<DbState>) -> bool {
    db(state)
        .get_settings()
        .map(|s| s.get("auto_snapshot").map(|v| v != "0").unwrap_or(true))
        .unwrap_or(true)
}

/// 启动一条 agent 配置：快照（如开启）→ 在对应 shell 新窗口的该空间目录下执行命令。
#[tauri::command]
fn launch(
    state: State<DbState>,
    processes: State<ProcessesState>,
    config: String,
) -> Result<u32, String> {
    let enabled = snapshot_enabled(&state);
    let (lc, dir) = db(&state).get_launch_config(&config).map_err(|e| e.to_string())?;
    let _ = hangar_core::snapshot_before_launch(&lc.workspace_id, &dir, enabled);
    start_config(&state, &processes, lc, dir)
}

/// 卡片「启动…」菜单：按名称 upsert 一条启动配置并立即运行。
/// 结果是配置固化在卡片上，状态跟踪/停止与手动配置完全一致。
#[tauri::command]
fn quick_launch(
    state: State<DbState>,
    processes: State<ProcessesState>,
    payload: QuickLaunch,
) -> Result<u32, String> {
    let enabled = snapshot_enabled(&state);
    let (lc, dir) = db(&state).upsert_launch_config(&payload).map_err(|e| e.to_string())?;
    let _ = hangar_core::snapshot_before_launch(&lc.workspace_id, &dir, enabled);
    start_config(&state, &processes, lc, dir)
}

/// 公共启动路径：单实例检查 → spawn → 注册表登记 + 记入活动流。
/// 实际 spawn（单实例检查 + 注册表登记）。GUI/托盘共用。
fn launch_locked(procs: &mut HashMap<String, Child>, lc: &LaunchConfig, dir: &str) -> Result<u32, String> {
    if let Some(child) = procs.get_mut(&lc.id) {
        if child.try_wait().map(|s| s.is_none()).unwrap_or(false) {
            return Err(format!("「{}」已在运行", lc.name));
        }
        procs.remove(&lc.id);
    }
    let child =
        launcher::spawn_in_shell(dir, &lc.command).map_err(|e| format!("启动失败: {e}"))?;
    let pid = child.id();
    procs.insert(lc.id.clone(), child);
    Ok(pid)
}

fn start_config(state: &State<DbState>, processes: &State<ProcessesState>, lc: LaunchConfig, dir: String) -> Result<u32, String> {
    let mut procs = processes.lock().expect("process registry poisoned");
    let pid = launch_locked(&mut procs, &lc, &dir)?;
    drop(procs);
    let _ = db(state).record_open(&lc.workspace_id, &format!("launch:{}", lc.name));
    Ok(pid)
}

/// 托盘点收藏空间：有配置直接起飞，无配置打开应用并定位浏览。
fn tray_launch(app: &tauri::AppHandle, ws_id: &str) {
    let db = match HangarDb::open() {
        Ok(d) => d,
        Err(_) => return,
    };
    let Ok(ws) = db.get(ws_id) else { return };
    let enabled = db
        .get_settings()
        .ok()
        .and_then(|s| s.get("auto_snapshot").cloned())
        .map(|v| v != "0")
        .unwrap_or(true);
    let _ = hangar_core::snapshot_before_launch(&ws.id, &ws.path, enabled);
    let procs_state = app.state::<ProcessesState>();
    let mut procs = procs_state.lock().expect("process registry poisoned");
    if let Some(lc) = ws.launch_configs.first().cloned() {
        if launch_locked(&mut procs, &lc, &ws.path).is_ok() {
            drop(procs);
            let _ = db.record_open(&ws.id, &format!("launch:{}", lc.name));
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
    } else {
        drop(procs);
        if let Some(w) = app.get_webview_window("main") {
            let _ = w.show();
            let _ = w.set_focus();
        }
        let _ = app.emit("browse-workspace", ws.id);
    }
}

/// 托盘事件分发。
fn handle_tray_event(app: &tauri::AppHandle, id: &str) {
    match id {
        "tray-show" => {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
                let _ = w.set_focus();
            }
        }
        "tray-quit" => app.exit(0),
        id if id.starts_with("ws:") => tray_launch(app, &id[3..]),
        _ => {}
    }
}

/// 重建托盘菜单（收藏空间动态列表）。
fn rebuild_tray_menu(app: &tauri::AppHandle) {
    let tray_state = app.state::<TrayState>();
    let guard = tray_state.lock().expect("tray state poisoned");
    let Some(tray) = guard.as_ref() else { return };
    let Ok(db) = HangarDb::open() else { return };
    let favs: Vec<Workspace> = db
        .list()
        .map(|l| l.into_iter().filter(|w| w.is_favorite).collect())
        .unwrap_or_default();

    let Ok(menu) = Menu::new(app) else { return };
    if let Ok(item) = MenuItem::with_id(app, "tray-show", "显示 Hangar", true, None::<&str>) {
        let _ = menu.append(&item);
    }
    if !favs.is_empty() {
        if let Ok(sep) = PredefinedMenuItem::separator(app) {
            let _ = menu.append(&sep);
        }
        for w in &favs {
            if let Ok(item) =
                MenuItem::with_id(app, &format!("ws:{}", w.id), &w.name, true, None::<&str>)
            {
                let _ = menu.append(&item);
            }
        }
    }
    if let Ok(sep) = PredefinedMenuItem::separator(app) {
        let _ = menu.append(&sep);
    }
    if let Ok(item) = MenuItem::with_id(app, "tray-quit", "退出 Hangar", true, None::<&str>) {
        let _ = menu.append(&item);
    }
    let _ = tray.set_menu(Some(menu));
}

/// 自启开关（设置面板用）。
#[tauri::command]
fn get_autostart(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch().is_enabled().map_err(|e| e.to_string())
}

#[tauri::command]
fn set_autostart(app: tauri::AppHandle, enable: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let al = app.autolaunch();
    if enable {
        al.enable().map_err(|e| e.to_string())
    } else {
        al.disable().map_err(|e| e.to_string())
    }
}

/// 常见编码 agent 名单（覆盖国内外主流），检测不到的可用设置里"我的 agent"手动补充。
const KNOWN_AGENTS: &[(&str, &str)] = &[
    ("claude", "Claude Code"),
    ("codex", "Codex CLI"),
    ("gemini", "Gemini CLI"),
    ("qwen", "Qwen Code"),
    ("zcode", "ZCode"),
    ("doubao", "豆包"),
    ("trae", "Trae"),
    ("copilot", "GitHub Copilot"),
    ("aider", "Aider"),
    ("opencode", "OpenCode"),
    ("crush", "Crush"),
    ("kilo", "Kilo Code"),
    ("workbuddy", "WorkBuddy"),
];

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentInfo {
    id: String,
    name: String,
    command: String,
}

#[tauri::command]
fn list_installed_agents() -> Vec<AgentInfo> {
    KNOWN_AGENTS
        .iter()
        .filter(|(cmd, _)| find_cmd_on_disk(cmd))
        .map(|(cmd, name)| AgentInfo {
            id: cmd.to_string(),
            name: name.to_string(),
            command: cmd.to_string(),
        })
        .collect()
}

/// 手动扫描 PATH 及 agent 常见安装位置，找 name / name.exe / name.cmd / name.bat。
/// 不依赖 where.exe，GUI 进程环境下更稳。
fn find_cmd_on_disk(cmd: &str) -> bool {
    let exts = ["", ".exe", ".cmd", ".bat"];
    let mut dirs: Vec<PathBuf> = Vec::new();
    if let Ok(path) = std::env::var("PATH") {
        dirs.extend(path.split(';').filter(|s| !s.is_empty()).map(PathBuf::from));
    }
    // agent 常见但不一定进 PATH 的安装位置
    if let Some(appdata) = std::env::var_os("APPDATA") {
        dirs.push(PathBuf::from(appdata).join("npm")); // npm 全局（claude/codex 的 .cmd 垫片）
    }
    if let Some(home) = std::env::var_os("USERPROFILE") {
        let home = PathBuf::from(home);
        dirs.push(home.join(".local").join("bin")); // Claude Code 原生安装
        dirs.push(home.join(".claude").join("local"));
        dirs.push(home.join("scoop").join("shims"));
        dirs.push(home.join("AppData").join("Roaming").join("npm"));
    }
    dirs.iter().any(|d| exts.iter().any(|e| d.join(format!("{cmd}{e}")).is_file()))
}

/// 所有被跟踪进程的状态：running / stopped（已退出的条目同时被清理）。
#[tauri::command]
fn process_statuses(processes: State<ProcessesState>) -> HashMap<String, String> {
    let mut procs = processes.lock().expect("process registry poisoned");
    let mut out = HashMap::new();
    let mut exited: Vec<String> = Vec::new();
    for (id, child) in procs.iter_mut() {
        match child.try_wait() {
            Ok(Some(_)) | Err(_) => exited.push(id.clone()),
            Ok(None) => {
                out.insert(id.clone(), "running".to_string());
            }
        }
    }
    for id in exited {
        procs.remove(&id);
    }
    out
}

/// 停止一条启动配置的进程树（taskkill /T，杀掉终端及其中的 agent）。
/// UI 层负责确认；这里只做动作。
#[tauri::command]
fn stop_process(processes: State<ProcessesState>, config: String) -> Result<(), String> {
    let mut procs = processes.lock().expect("process registry poisoned");
    let Some(child) = procs.get_mut(&config) else {
        return Ok(()); // 不在注册表中（已退出或重启后），无事可做
    };
    let pid = child.id();
    let _ = std::process::Command::new("taskkill")
        .args(["/PID", &pid.to_string(), "/T", "/F"])
        .output();
    procs.remove(&config);
    Ok(())
}

/// 确认弹窗"记住选择"的持久化入口。
#[tauri::command]
fn set_launch_confirm_skip(state: State<DbState>, config: String, skip: bool) -> Result<(), String> {
    db(&state)
        .set_launch_confirm_skip(&config, skip)
        .map_err(|e| e.to_string())
}

pub fn run() {
    let hangar = HangarDb::open().expect("无法打开 Hangar 数据库");
    // 本地 REST API（127.0.0.1:7333）：独立连接同一 SQLite（WAL），线程内常驻
    {
        let rest_db = std::sync::Arc::new(std::sync::Mutex::new(
            HangarDb::open().expect("无法打开 REST 用的数据库连接"),
        ));
        hangar_core::rest::spawn(rest_db);
    }
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(
            tauri_plugin_global_shortcut::Builder::new()
                .with_shortcuts(["alt+h"])
                .expect("注册全局快捷键 Alt+H 失败")
                .with_handler(|app, _shortcut, event| {
                    if event.state() == tauri_plugin_global_shortcut::ShortcutState::Pressed {
                        if let Some(w) = app.get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.unminimize();
                            let _ = w.set_focus();
                            let _ = w.emit("palette-toggle", ());
                        }
                    }
                })
                .build(),
        )
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        .setup(|app| {
            // 托盘：显示 / 收藏空间（动态重建） / 退出
            let show = MenuItem::with_id(app, "tray-show", "显示 Hangar", true, None::<&str>)?;
            let quit = MenuItem::with_id(app, "tray-quit", "退出 Hangar", true, None::<&str>)?;
            let sep = PredefinedMenuItem::separator(app)?;
            let menu = Menu::with_items(app, &[&show, &sep, &quit])?;
            let tray = TrayIconBuilder::with_id("main-tray")
                .icon(app.default_window_icon().expect("missing bundle icon").clone())
                .tooltip("Hangar — agent 工作空间总台")
                .menu(&menu)
                .show_menu_on_left_click(false)
                .on_menu_event(|app, event| handle_tray_event(app, event.id().as_ref()))
                .on_tray_icon_event(|tray, event| {
                    if let TrayIconEvent::Click {
                        button: MouseButton::Left,
                        button_state: MouseButtonState::Up,
                        ..
                    } = event
                    {
                        if let Some(w) = tray.app_handle().get_webview_window("main") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;
            app.manage::<TrayState>(Mutex::new(Some(tray)));
            rebuild_tray_menu(app.handle());
            Ok(())
        })
        .on_window_event(|window, event| {
            // 关窗 = 隐藏到托盘（设置可关）；隐藏前重建菜单，收藏列表保持最新
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                let hide = {
                    let s = window.app_handle().state::<DbState>();
                    let g = s.lock().expect("db poisoned");
                    g.get_settings()
                        .ok()
                        .and_then(|m| m.get("close_to_tray").cloned())
                        .map(|v| v != "0")
                        .unwrap_or(true)
                };
                if hide {
                    api.prevent_close();
                    rebuild_tray_menu(window.app_handle());
                    let _ = window.hide();
                }
            }
        })
        .manage(Mutex::new(hangar))
        .manage(Mutex::new(HashMap::<String, Child>::new()))
        .invoke_handler(tauri::generate_handler![
            list_workspaces,
            add_workspace,
            update_workspace,
            set_favorite,
            delete_workspace,
            record_open,
            open_in_explorer,
            open_in_terminal,
            list_groups,
            create_group,
            delete_group,
            set_workspace_group,
            scan_import_root,
            import_workspaces,
            launch,
            process_statuses,
            stop_process,
            set_launch_confirm_skip,
            git_statuses,
            get_settings,
            set_settings,
            open_in_editor,
            quick_launch,
            list_installed_agents,
            list_dir,
            open_entry,
            read_text_file,
            write_text_file,
            list_snapshots,
            restore_snapshot,
            delete_snapshot,
            activity_feed,
            get_autostart,
            set_autostart
        ])
        .run(tauri::generate_context!())
        .expect("Hangar 运行出错");
}
