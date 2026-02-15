#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod kanata;
mod tray;

fn main() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            Some(vec![]),
        ))
        .plugin(tauri_plugin_store::Builder::default().build())
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
            commands::browse_folder,
            commands::open_install_dir,
            commands::open_config_in_editor,
            commands::validate_timestamp_format,
            commands::get_key_bindings,
        ])
        .setup(|app| {
            tray::setup(app)?;
            kanata::setup(app)?;
            Ok(())
        })
        .on_window_event(|window, event| {
            // ×ボタンでトレイに最小化（終了しない）
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
            }
        })
        .build(tauri::generate_context!())
        .expect("error building tauri application");

    app.run(|_app_handle, event| {
        // Prevent app exit when all windows are closed (tray app)
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            api.prevent_exit();
        }
    });
}
