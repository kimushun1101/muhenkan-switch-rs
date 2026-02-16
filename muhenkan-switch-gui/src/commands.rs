use muhenkan_switch_config::{self as config, Config};
use serde::Serialize;
use std::path::PathBuf;
use tauri::State;

use crate::kanata::KanataManager;

// ── Config commands ──

fn resolve_config_path() -> PathBuf {
    config::config_path().unwrap_or_else(|| {
        // Default: exe dir / config.toml
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("config.toml")))
            .unwrap_or_else(|| PathBuf::from("config.toml"))
    })
}

#[tauri::command]
pub fn get_config() -> Result<Config, String> {
    config::load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn save_config(config: Config) -> Result<(), String> {
    let errors = config::validate(&config);
    if !errors.is_empty() {
        return Err(errors.join("\n"));
    }
    let path = resolve_config_path();
    config::save(&path, &config).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reset_config() -> Result<Config, String> {
    config::load().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn default_config() -> Config {
    config::default_config()
}

// ── Kanata commands ──

#[derive(Serialize, Clone)]
pub struct KanataStatus {
    pub running: bool,
    pub pid: Option<u32>,
}

#[tauri::command]
pub fn get_kanata_status(manager: State<KanataManager>) -> KanataStatus {
    let (running, pid) = manager.status();
    KanataStatus { running, pid }
}

#[tauri::command]
pub fn start_kanata(manager: State<KanataManager>) -> Result<(), String> {
    manager.start().map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub fn stop_kanata(manager: State<KanataManager>) -> Result<(), String> {
    manager.stop().map_err(|e| format!("{:#}", e))
}

#[tauri::command]
pub fn restart_kanata(manager: State<KanataManager>) -> Result<(), String> {
    manager.restart().map_err(|e| format!("{:#}", e))
}

// ── Process list (for app selection) ──

#[derive(Serialize)]
pub struct ProcessInfo {
    pub name: String,
    pub pid: u32,
}

#[tauri::command]
pub fn get_running_processes() -> Result<Vec<ProcessInfo>, String> {
    get_processes_impl().map_err(|e| e.to_string())
}

#[cfg(target_os = "windows")]
fn get_processes_impl() -> anyhow::Result<Vec<ProcessInfo>> {
    use std::collections::HashSet;
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };

    let mut processes = Vec::new();
    let mut seen = HashSet::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)
            .map_err(|e| anyhow::anyhow!("Failed to create snapshot: {}", e))?;
        let mut entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };

        if Process32FirstW(snapshot, &mut entry).is_ok() {
            loop {
                let exe_len = entry
                    .szExeFile
                    .iter()
                    .position(|&c| c == 0)
                    .unwrap_or(entry.szExeFile.len());
                let name = OsString::from_wide(&entry.szExeFile[..exe_len])
                    .to_string_lossy()
                    .to_string();

                if !seen.contains(&name) {
                    seen.insert(name.clone());
                    processes.push(ProcessInfo {
                        name,
                        pid: entry.th32ProcessID,
                    });
                }

                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
    }

    processes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(processes)
}

#[cfg(not(target_os = "windows"))]
fn get_processes_impl() -> anyhow::Result<Vec<ProcessInfo>> {
    // Linux/macOS: use ps command
    let output = std::process::Command::new("ps")
        .args(["-eo", "pid,comm"])
        .output()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut processes = Vec::new();
    let mut seen = std::collections::HashSet::new();

    for line in stdout.lines().skip(1) {
        let parts: Vec<&str> = line.trim().splitn(2, char::is_whitespace).collect();
        if parts.len() == 2 {
            let pid: u32 = parts[0].trim().parse().unwrap_or(0);
            let name = parts[1].trim().to_string();
            if !seen.contains(&name) {
                seen.insert(name.clone());
                processes.push(ProcessInfo { name, pid });
            }
        }
    }

    processes.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    Ok(processes)
}

// ── Autostart ──

#[tauri::command]
pub fn get_autostart_enabled(app: tauri::AppHandle) -> Result<bool, String> {
    use tauri_plugin_autostart::ManagerExt;
    app.autolaunch()
        .is_enabled()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_autostart_enabled(app: tauri::AppHandle, enabled: bool) -> Result<(), String> {
    use tauri_plugin_autostart::ManagerExt;
    let autostart = app.autolaunch();
    if enabled {
        autostart.enable().map_err(|e| e.to_string())
    } else {
        autostart.disable().map_err(|e| e.to_string())
    }
}

// ── Utility commands ──

#[tauri::command]
pub async fn browse_folder(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog().file().pick_folder(move |path| {
        let _ = tx.send(path.map(|p| p.to_string()));
    });
    rx.recv()
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_install_dir() -> Result<(), String> {
    let dir = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.to_path_buf()))
        .ok_or_else(|| "Cannot determine install directory".to_string())?;
    open::that(&dir).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_config_in_editor() -> Result<(), String> {
    let path = resolve_config_path();
    open::that(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn validate_timestamp_format(
    format: String,
    delimiter: String,
    position: String,
) -> Result<String, String> {
    if format.is_empty() {
        return Err("フォーマットを入力してください".to_string());
    }
    let now = chrono::Local::now();
    use std::fmt::Write;
    let mut ts = String::new();
    write!(ts, "{}", now.format(&format))
        .map_err(|_| "無効なフォーマット文字列です".to_string())?;
    let (stem, ext) = ("FileName", ".txt");
    let preview = if position == "after" {
        format!("{}{}{}{}", stem, delimiter, ts, ext)
    } else {
        format!("{}{}{}{}", ts, delimiter, stem, ext)
    };
    Ok(preview)
}
