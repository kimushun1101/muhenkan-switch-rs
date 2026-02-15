use anyhow::Result;
use std::process::Command;

use crate::config::{self, Config};

pub fn run(target: &str, config: &Config) -> Result<()> {
    let app = config::get_value(&config.apps, target, "App")?;
    activate_window(app)
}

#[cfg(target_os = "windows")]
fn activate_window(app: &str) -> Result<()> {
    // PowerShell でプロセス名からウィンドウをアクティブ化
    let script = format!(
        r#"
        $proc = Get-Process -Name '{}' -ErrorAction SilentlyContinue | Select-Object -First 1
        if ($proc -and $proc.MainWindowHandle -ne 0) {{
            Add-Type -TypeDefinition '
                using System;
                using System.Runtime.InteropServices;
                public class Win32 {{
                    [DllImport("user32.dll")]
                    public static extern bool SetForegroundWindow(IntPtr hWnd);
                    [DllImport("user32.dll")]
                    public static extern bool ShowWindow(IntPtr hWnd, int nCmdShow);
                }}
            '
            [Win32]::ShowWindow($proc.MainWindowHandle, 9)
            [Win32]::SetForegroundWindow($proc.MainWindowHandle)
        }}
        "#,
        app
    );
    Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()?;
    Ok(())
}

#[cfg(target_os = "linux")]
fn activate_window(app: &str) -> Result<()> {
    // wmctrl でウィンドウをアクティブ化
    let result = Command::new("wmctrl").args(["-a", app]).output();

    match result {
        Ok(_) => Ok(()),
        Err(_) => {
            // wmctrl がない場合は xdotool を試行
            Command::new("xdotool")
                .args(["search", "--name", app, "windowactivate"])
                .output()?;
            Ok(())
        }
    }
}

#[cfg(target_os = "macos")]
fn activate_window(app: &str) -> Result<()> {
    // osascript でアプリをアクティブ化（未検証）
    Command::new("osascript")
        .args(["-e", &format!(r#"tell application "{}" to activate"#, app)])
        .output()?;
    Ok(())
}
