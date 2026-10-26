# -*- coding: utf-8 -*-
# 托盘补丁 A：Cargo + lib.rs（imports/托盘状态/启动重构/托盘逻辑/自启命令）
import io

LIB = r'D:\个人文档\code\hangar\src-tauri\src\lib.rs'
CARGO = r'D:\个人文档\code\hangar\src-tauri\Cargo.toml'

def patch(path, pairs, tag):
    t = io.open(path, encoding='utf-8').read()
    for old, new in pairs:
        if old not in t:
            print(f'  SKIP(already?) [{tag}] {old[:50]!r}')
            continue
        t = t.replace(old, new, 1)
    io.open(path, 'w', encoding='utf-8').write(t)
    print('ok', tag)

# 1) Cargo
patch(CARGO, [
    ('tauri = { version = "2", features = ["protocol-asset"] }',
     'tauri = { version = "2", features = ["protocol-asset", "tray-icon", "image-png", "image-ico"] }'),
    ('tauri-plugin-global-shortcut = "2"',
     'tauri-plugin-global-shortcut = "2"\ntauri-plugin-autostart = "2"'),
], 'cargo')

# 2) imports
patch(LIB, [
    ('use tauri::{Emitter, Manager, State};',
     'use tauri::{\n    menu::{Menu, MenuItem, PredefinedMenuItem},\n    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},\n    Emitter, Manager, State, WindowEvent,\n};'),
], 'imports')

# 3) TrayState
patch(LIB, [
    ('type ProcessesState = Mutex<HashMap<String, Child>>;',
     'type ProcessesState = Mutex<HashMap<String, Child>>;\n/// 托盘图标句柄（重建菜单时用）\ntype TrayState = Mutex<Option<tauri::tray::TrayIcon<tauri::Wry>>>;'),
], 'traystate')

# 4) launch_locked 抽取 + start_config 委托 + 托盘启动/菜单/自启命令
patch(LIB, [
    ('''fn start_config(state: &State<DbState>, processes: &State<ProcessesState>, lc: LaunchConfig, dir: String) -> Result<u32, String> {
    let mut procs = processes.lock().expect("process registry poisoned");
    if let Some(child) = procs.get_mut(&lc.id) {
        if child.try_wait().map(|s| s.is_none()).unwrap_or(false) {
            return Err(format!("「{}」已在运行", lc.name));
        }
        procs.remove(&lc.id);
    }
    let child =
        launcher::spawn_in_shell(&lc.shell, &dir, &lc.command).map_err(|e| format!("启动失败: {e}"))?;
    let pid = child.id();
    procs.insert(lc.id.clone(), child);
    let _ = db(state).record_open(&lc.workspace_id, &format!("launch:{}", lc.name));
    Ok(pid)
}''',
'''/// 实际 spawn（单实例检查 + 注册表登记）。GUI/托盘共用。
fn launch_locked(procs: &mut HashMap<String, Child>, lc: &LaunchConfig, dir: &str) -> Result<u32, String> {
    if let Some(child) = procs.get_mut(&lc.id) {
        if child.try_wait().map(|s| s.is_none()).unwrap_or(false) {
            return Err(format!("「{}」已在运行", lc.name));
        }
        procs.remove(&lc.id);
    }
    let child =
        launcher::spawn_in_shell(&lc.shell, dir, &lc.command).map_err(|e| format!("启动失败: {e}"))?;
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
}'''),
], 'lib-part-a')
