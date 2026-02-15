use anyhow::Result;
use std::process::Command;

#[cfg(target_os = "windows")]
pub fn simulate_copy() -> Result<()> {
    let script = r#"
        Add-Type -AssemblyName System.Windows.Forms
        [System.Windows.Forms.SendKeys]::SendWait('^c')
    "#;
    Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn simulate_copy() -> Result<()> {
    Command::new("xdotool")
        .args(["key", "ctrl+c"])
        .output()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn simulate_copy() -> Result<()> {
    Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "c" using command down"#,
        ])
        .output()?;
    Ok(())
}

#[cfg(target_os = "windows")]
pub fn simulate_paste() -> Result<()> {
    let script = r#"
        Add-Type -AssemblyName System.Windows.Forms
        [System.Windows.Forms.SendKeys]::SendWait('^v')
    "#;
    Command::new("powershell")
        .args(["-NoProfile", "-Command", script])
        .output()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn simulate_paste() -> Result<()> {
    Command::new("xdotool")
        .args(["key", "ctrl+v"])
        .output()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn simulate_paste() -> Result<()> {
    Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "v" using command down"#,
        ])
        .output()?;
    Ok(())
}
