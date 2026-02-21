use std::sync::Mutex;
use std::time::Instant;

use tauri::menu::{CheckMenuItemBuilder, MenuBuilder, MenuItemBuilder, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{AppHandle, Manager};

use crate::kanata::KanataManager;

pub fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle();

    build_tray(handle)?;

    Ok(())
}

fn build_tray(handle: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let status_item =
        MenuItemBuilder::with_id("kanata_status", "キー割当（kanata）: 停止中")
            .enabled(false)
            .build(handle)?;
    let start_item =
        MenuItemBuilder::with_id("kanata_start", "キー割当（kanata）を開始").build(handle)?;
    let stop_item =
        MenuItemBuilder::with_id("kanata_stop", "キー割当（kanata）を停止").build(handle)?;
    let restart_item =
        MenuItemBuilder::with_id("kanata_restart", "キー割当（kanata）を再起動").build(handle)?;
    let sep1 = PredefinedMenuItem::separator(handle)?;
    let settings_item =
        MenuItemBuilder::with_id("settings", "設定...").build(handle)?;
    let open_config_item =
        MenuItemBuilder::with_id("open_config", "config.toml を開く").build(handle)?;
    let open_dir_item = MenuItemBuilder::with_id("open_dir", "インストール先を開く")
        .build(handle)?;
    let sep2 = PredefinedMenuItem::separator(handle)?;
    let autostart_item =
        CheckMenuItemBuilder::with_id("autostart", "ログイン時に自動起動")
            .build(handle)?;
    let sep3 = PredefinedMenuItem::separator(handle)?;
    let quit_item = MenuItemBuilder::with_id("quit", "終了").build(handle)?;

    let menu = MenuBuilder::new(handle)
        .item(&status_item)
        .item(&start_item)
        .item(&stop_item)
        .item(&restart_item)
        .item(&sep1)
        .item(&settings_item)
        .item(&open_config_item)
        .item(&open_dir_item)
        .item(&sep2)
        .item(&autostart_item)
        .item(&sep3)
        .item(&quit_item)
        .build()?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("muhenkan-switch")
        .on_menu_event(move |app, event| {
            let id = event.id().as_ref();
            match id {
                "kanata_start" => {
                    let manager = app.state::<KanataManager>();
                    let _ = manager.start();
                }
                "kanata_stop" => {
                    let manager = app.state::<KanataManager>();
                    let _ = manager.stop();
                }
                "kanata_restart" => {
                    let manager = app.state::<KanataManager>();
                    let _ = manager.restart();
                }
                "settings" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
                "open_config" => {
                    let _ = crate::commands::open_config_in_editor();
                }
                "open_dir" => {
                    let _ = crate::commands::open_install_dir();
                }
                "autostart" => {
                    use tauri_plugin_autostart::ManagerExt;
                    if let Ok(enabled) = app.autolaunch().is_enabled() {
                        if enabled {
                            let _ = app.autolaunch().disable();
                        } else {
                            let _ = app.autolaunch().enable();
                        }
                    }
                }
                "quit" => {
                    let manager = app.state::<KanataManager>();
                    let _ = manager.stop();
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event({
            let last_click = Mutex::new(Instant::now() - std::time::Duration::from_secs(1));
            move |tray, event| {
                if let tauri::tray::TrayIconEvent::Click {
                    button: tauri::tray::MouseButton::Left,
                    ..
                } = event
                {
                    // デバウンス: 500ms 以内の重複クリックを無視
                    let mut last = last_click.lock().unwrap();
                    if last.elapsed().as_millis() < 500 {
                        return;
                    }
                    *last = Instant::now();

                    let app = tray.app_handle();
                    if let Some(window) = app.get_webview_window("main") {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.hide();
                        } else {
                            let _ = window.show();
                            let _ = window.set_focus();
                        }
                    }
                }
            }
        })
        .build(handle)?;

    Ok(())
}
