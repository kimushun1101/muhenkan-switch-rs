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
    // wmctrl でウィンドウをアクティブ化
    let result = Command::new("wmctrl").args(["-a", app]).output();

    let activated = match result {
        Ok(output) if output.status.success() => true,
        _ => {
            // wmctrl がない場合は xdotool を試行
            let output = Command::new("xdotool")
                .args(["search", "--name", app, "windowactivate"])
                .output()?;
            output.status.success()
        }
    };

    if !activated {
        if let Some(cmd) = launch {
            Command::new("sh")
                .args(["-c", cmd])
                .spawn()?;
        }
    }

    Ok(())
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
