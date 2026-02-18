use anyhow::Result;
use std::process::Command;

use crate::config::Config;

pub fn run(target: &str, config: &Config) -> Result<()> {
    let entry = config
        .apps
        .get(target)
        .ok_or_else(|| anyhow::anyhow!("App '{}' is not defined in config.toml", target))?;

    let process_name = entry.process();
    let command = entry.command();

    activate_window(process_name, command)
}

#[cfg(target_os = "windows")]
fn activate_window(app: &str, launch: Option<&str>) -> Result<()> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W,
        TH32CS_SNAPPROCESS,
    };
    use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, IsWindowVisible,
        SetForegroundWindow, ShowWindow, SW_RESTORE,
    };
    use windows::core::BOOL;
    use windows::Win32::Foundation::{HWND, LPARAM};

    // --- Step 1: Find PIDs matching the process name ---
    let app_lower = app.to_ascii_lowercase();
    let mut pids = Vec::new();

    unsafe {
        let snapshot = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0)?;
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
                let exe_name = OsString::from_wide(&entry.szExeFile[..exe_len])
                    .to_string_lossy()
                    .to_ascii_lowercase();
                // Match with or without .exe extension
                if exe_name == app_lower || exe_name == format!("{}.exe", app_lower) {
                    pids.push(entry.th32ProcessID);
                }
                if Process32NextW(snapshot, &mut entry).is_err() {
                    break;
                }
            }
        }
        let _ = windows::Win32::Foundation::CloseHandle(snapshot);
    }

    if pids.is_empty() {
        // Process not found — launch if configured
        if let Some(cmd) = launch {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            Command::new("cmd")
                .args(["/C", "start", "", cmd])
                .creation_flags(CREATE_NO_WINDOW)
                .spawn()?;
        }
        return Ok(());
    }

    // --- Step 2: Find a visible top-level window belonging to one of the PIDs ---
    struct CallbackData {
        pids: Vec<u32>,
        hwnd: Option<HWND>,
    }

    unsafe extern "system" fn enum_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let data = &mut *(lparam.0 as *mut CallbackData);
        let mut pid: u32 = 0;
        GetWindowThreadProcessId(hwnd, Some(&mut pid));
        if data.pids.contains(&pid) && IsWindowVisible(hwnd).as_bool() {
            data.hwnd = Some(hwnd);
            return BOOL(0); // stop enumeration
        }
        BOOL(1) // continue
    }

    let mut data = CallbackData {
        pids,
        hwnd: None,
    };

    unsafe {
        let _ = EnumWindows(
            Some(enum_callback),
            LPARAM(&mut data as *mut CallbackData as isize),
        );
    }

    let hwnd = match data.hwnd {
        Some(h) => h,
        None => {
            // Window not found — launch if configured
            if let Some(cmd) = launch {
                use std::os::windows::process::CommandExt;
                const CREATE_NO_WINDOW: u32 = 0x08000000;
                Command::new("cmd")
                    .args(["/C", "start", "", cmd])
                    .creation_flags(CREATE_NO_WINDOW)
                    .spawn()?;
            }
            return Ok(());
        }
    };

    // --- Step 3: Activate the window ---
    unsafe {
        let fg_hwnd = GetForegroundWindow();
        let fg_thread = GetWindowThreadProcessId(fg_hwnd, None);
        let cur_thread = GetCurrentThreadId();

        let attached = if fg_thread != cur_thread {
            AttachThreadInput(cur_thread, fg_thread, true).as_bool()
        } else {
            false
        };

        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }
        let _ = SetForegroundWindow(hwnd);

        if attached {
            let _ = AttachThreadInput(cur_thread, fg_thread, false);
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn activate_window(app: &str, launch: Option<&str>) -> Result<()> {
    if is_wayland() {
        activate_window_wayland(app, launch)
    } else {
        activate_window_x11(app, launch)
    }
}

/// Wayland セッション判定
#[cfg(target_os = "linux")]
fn is_wayland() -> bool {
    std::env::var("WAYLAND_DISPLAY").is_ok()
        || std::env::var("XDG_SESSION_TYPE")
            .map(|v| v == "wayland")
            .unwrap_or(false)
}

/// Wayland 環境でのウィンドウアクティブ化
/// GNOME Shell の Eval API は制限されているため、以下の順で試行:
/// 1. xdotool (XWayland 経由で動く場合がある)
/// 2. wmctrl -x (XWayland 経由)
/// 3. アプリを起動（既存インスタンスがあれば D-Bus 経由でフォーカスされるアプリもある）
#[cfg(target_os = "linux")]
fn activate_window_wayland(app: &str, launch: Option<&str>) -> Result<()> {
    // XWayland 経由で動く可能性があるので X11 ツールを試す
    let activated = try_wmctrl(app)
        || try_xdotool(app, "--class")
        || try_xdotool(app, "--name");

    if !activated {
        eprintln!(
            "Warning: Wayland ではウィンドウのアクティブ化ができません。\
             X11 セッション（「Ubuntu on Xorg」）への切り替えを推奨します。"
        );
        if let Some(cmd) = launch {
            if let Err(e) = Command::new("sh").args(["-c", cmd]).spawn() {
                eprintln!("Warning: failed to launch '{}': {}", cmd, e);
            }
        }
    }

    Ok(())
}

/// X11 環境でのウィンドウアクティブ化
/// 1. wmctrl -x -a (WM_CLASS でマッチ — タイトルより安定)
/// 2. xdotool search --class (WM_CLASS でマッチ)
/// 3. xdotool search --name (ウィンドウタイトルでマッチ)
#[cfg(target_os = "linux")]
fn activate_window_x11(app: &str, launch: Option<&str>) -> Result<()> {
    let activated = try_wmctrl(app)
        || try_xdotool(app, "--class")
        || try_xdotool(app, "--name");

    if !activated {
        if let Some(cmd) = launch {
            if let Err(e) = Command::new("sh").args(["-c", cmd]).spawn() {
                eprintln!("Warning: failed to launch '{}': {}", cmd, e);
            }
        }
    }

    Ok(())
}

#[cfg(target_os = "linux")]
fn try_wmctrl(app: &str) -> bool {
    // -x: WM_CLASS でマッチ（ウィンドウタイトルより安定）
    Command::new("wmctrl")
        .args(["-x", "-a", app])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(target_os = "linux")]
fn try_xdotool(app: &str, search_flag: &str) -> bool {
    // --onlyvisible: 不可視の内部ウィンドウを除外（これがないと GNOME で失敗する）
    let result = Command::new("xdotool")
        .args(["search", "--onlyvisible", search_flag, app])
        .output();
    match result {
        Ok(output) if output.status.success() => {
            let stdout = String::from_utf8_lossy(&output.stdout);
            if let Some(wid) = stdout.lines().next() {
                Command::new("xdotool")
                    .args(["windowactivate", "--sync", wid])
                    .output()
                    .map(|o| o.status.success())
                    .unwrap_or(false)
            } else {
                false
            }
        }
        _ => false,
    }
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    use super::*;

    #[test]
    fn try_wmctrl_nonexistent_app_returns_false() {
        // 存在しないアプリ名で wmctrl を試行 → false（パニックしない）
        assert!(!try_wmctrl("__nonexistent_app_muhenkan_test_99999__"));
    }

    #[test]
    fn try_xdotool_class_nonexistent_returns_false() {
        assert!(!try_xdotool(
            "__nonexistent_app_muhenkan_test_99999__",
            "--class"
        ));
    }

    #[test]
    fn try_xdotool_name_nonexistent_returns_false() {
        assert!(!try_xdotool(
            "__nonexistent_app_muhenkan_test_99999__",
            "--name"
        ));
    }

    #[test]
    fn activate_window_nonexistent_no_launch_returns_ok() {
        // 存在しないアプリ、launch なし → エラーにならず Ok
        let result = activate_window("__nonexistent_app_muhenkan_test_99999__", None);
        assert!(result.is_ok());
    }

    #[test]
    fn activate_window_nonexistent_with_bad_launch_returns_ok() {
        // launch コマンドが失敗しても eprintln で警告のみ、Ok を返す
        let result = activate_window(
            "__nonexistent_app_muhenkan_test_99999__",
            Some("/bin/__nonexistent_command_99999__"),
        );
        assert!(result.is_ok());
    }

    #[test]
    fn is_wayland_returns_bool() {
        // Wayland 判定がパニックしないことを確認（結果は環境依存）
        let _ = is_wayland();
    }

    #[test]
    fn run_missing_app_errors() {
        let config = Config {
            search: Default::default(),
            folders: Default::default(),
            apps: Default::default(),
            timestamp: Default::default(),
        };
        let result = run("nonexistent", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not defined"));
    }
}

#[cfg(target_os = "macos")]
fn activate_window(app: &str, launch: Option<&str>) -> Result<()> {
    // osascript の activate は未起動アプリも自動起動する
    // launch が設定されていればそちらを優先
    let target = launch.unwrap_or(app);
    Command::new("osascript")
        .args(["-e", &format!(r#"tell application "{}" to activate"#, target)])
        .output()?;
    Ok(())
}
