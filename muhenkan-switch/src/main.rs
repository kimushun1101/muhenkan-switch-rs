#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod kanata;
mod tray;

fn main() {
    // windows_subsystem = "windows" では stderr が見えないため、パニック時にファイルへ記録
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let Some(log_path) = std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("panic.log")))
        {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(log_path)
            {
                use std::io::Write;
                let _ = writeln!(f, "{}", info);
                let _ = writeln!(f, "{}", std::backtrace::Backtrace::force_capture());
            }
        }
        default_hook(info);
    }));

    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(kanata::KanataManager::new())
        .invoke_handler(tauri::generate_handler![
            commands::get_config,
            commands::save_config,
            commands::reset_config,
            commands::default_config,
            commands::get_kanata_status,
            commands::start_kanata,
            commands::stop_kanata,
            commands::restart_kanata,
            commands::get_running_processes,
            commands::get_autostart_enabled,
            commands::set_autostart_enabled,
            commands::get_config_path,
            commands::get_app_version,
            commands::quit_app,
            commands::browse_folder,
            commands::open_install_dir,
            commands::open_config_in_editor,
            commands::open_help_window,
            commands::validate_timestamp_format,
            commands::check_update,
            commands::install_update,
            commands::get_install_type,
        ])
        .setup(|app| {
            // 初回起動時: exe 同梱ディレクトリに config.toml がなければデフォルト設定を生成
            if let Ok(exe_path) = std::env::current_exe() {
                if let Some(exe_dir) = exe_path.parent() {
                    let config_path = exe_dir.join("config.toml");
                    if !config_path.exists() {
                        let default = muhenkan_switch_config::default_config();
                        if let Err(e) = muhenkan_switch_config::save(&config_path, &default) {
                            eprintln!("[setup] config.toml の生成に失敗: {:#}", e);
                        }
                    }
                }
            }
            tray::setup(app)?;
            kanata::setup(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                if window.label() == "main" {
                    // メインウィンドウは×ボタンでトレイに最小化（終了しない）
                    api.prevent_close();
                    let _ = window.hide();
                }
                // それ以外（help 等）は通常通り閉じる
            }
        })
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(|_app_handle, event| {
        // code == None: 全ウィンドウ閉じによる自動終了 → トレイ常駐のため阻止
        // code == Some(_): app.exit() による意図的な終了 → 許可
        if let tauri::RunEvent::ExitRequested { code, api, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
