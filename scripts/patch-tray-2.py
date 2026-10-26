# -*- coding: utf-8 -*-
# 托盘补丁第二段：run() 里的插件注册/托盘构建/关窗拦截 + 命令注册
import io

LIB = r'D:\个人文档\code\hangar\src-tauri\src\lib.rs'

def patch(path, pairs, tag):
    t = io.open(path, encoding='utf-8').read()
    for old, new in pairs:
        assert old in t, f'MISS [{tag}] {old[:60]!r}'
        t = t.replace(old, new, 1)
    io.open(path, 'w', encoding='utf-8').write(t)
    print('ok', tag)

# 1) autostart 插件 + setup 托盘 + 关窗拦截
patch(LIB, [
    ('''        .plugin(
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
        )''',
'''        .plugin(
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
        })'''),

    # 2) 命令注册
    ('''            list_snapshots,
            restore_snapshot,
            delete_snapshot,
            activity_feed
        ])''',
'''            list_snapshots,
            restore_snapshot,
            delete_snapshot,
            activity_feed,
            get_autostart,
            set_autostart
        ])'''),
], 'lib-run')

# 3) App.vue：监听 browse-workspace（托盘点无配置的收藏空间）
APP = r'D:\个人文档\code\hangar\src\App.vue'
t = io.open(APP, encoding='utf-8').read()
old = '''  // 全局快捷键 Alt+H（后端发来）→ 命令面板开关'''
add = '''  // 托盘点收藏空间（无配置）→ 定位浏览
  listen("browse-workspace", (e) => {
    const id = (e as unknown as { payload: string }).payload;
    const ws = store.workspaces.find((w) => w.id === id);
    if (ws) browsing.value = ws;
  }).then((un) => (unlistenBrowse = un))
    .catch(() => void 0);
  // 全局快捷键 Alt+H（后端发来）→ 命令面板开关'''
assert old in t, 'browse anchor missing'
t = t.replace(old, add, 1)
t = t.replace('let unlistenPal: (() => void) | undefined;',
              'let unlistenPal: (() => void) | undefined;\nlet unlistenBrowse: (() => void) | undefined;', 1)
t = t.replace('''  if (unlistenPal) unlistenPal();
});''', '''  if (unlistenPal) unlistenPal();
  if (unlistenBrowse) unlistenBrowse();
});''', 1)
io.open(APP, 'w', encoding='utf-8').write(t)
print('ok app-listener')

# 4) utils：DEFAULT_SETTINGS 加 close_to_tray
p = r'D:\个人文档\code\hangar\src\lib\utils.ts'
t = io.open(p, encoding='utf-8').read()
t = t.replace('''  editor_cmd: "code {path}",
  custom_agents: "",
};''', '''  editor_cmd: "code {path}",
  custom_agents: "",
  close_to_tray: "1",
};''', 1)
io.open(p, 'w', encoding='utf-8').write(t)
print('ok utils')
