use anyhow::Result;

#[cfg(target_os = "windows")]
pub fn simulate_copy() -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_C;
    send_ctrl_key(VK_C)
}

#[cfg(target_os = "windows")]
pub fn simulate_paste() -> Result<()> {
    use windows::Win32::UI::Input::KeyboardAndMouse::VK_V;
    send_ctrl_key(VK_V)
}

/// Send Ctrl+<key> via Win32 SendInput (replaces PowerShell invocation).
#[cfg(target_os = "windows")]
fn send_ctrl_key(vk: windows::Win32::UI::Input::KeyboardAndMouse::VIRTUAL_KEY) -> Result<()> {
    use std::mem;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_CONTROL,
    };

    unsafe {
        let mut inputs = [INPUT::default(), INPUT::default(), INPUT::default(), INPUT::default()];

        // Ctrl down
        inputs[0].r#type = INPUT_KEYBOARD;
        inputs[0].Anonymous.ki = KEYBDINPUT {
            wVk: VK_CONTROL,
            ..Default::default()
        };

        // Key down
        inputs[1].r#type = INPUT_KEYBOARD;
        inputs[1].Anonymous.ki = KEYBDINPUT {
            wVk: vk,
            ..Default::default()
        };

        // Key up
        inputs[2].r#type = INPUT_KEYBOARD;
        inputs[2].Anonymous.ki = KEYBDINPUT {
            wVk: vk,
            dwFlags: KEYEVENTF_KEYUP,
            ..Default::default()
        };

        // Ctrl up
        inputs[3].r#type = INPUT_KEYBOARD;
        inputs[3].Anonymous.ki = KEYBDINPUT {
            wVk: VK_CONTROL,
            dwFlags: KEYEVENTF_KEYUP,
            ..Default::default()
        };

        let sent = SendInput(&inputs, mem::size_of::<INPUT>() as i32);
        if sent != 4 {
            anyhow::bail!("SendInput failed: only {} of 4 inputs sent", sent);
        }
    }
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn simulate_copy() -> Result<()> {
    use std::process::Command;
    Command::new("xdotool")
        .args(["key", "ctrl+c"])
        .output()?;
    Ok(())
}

#[cfg(target_os = "linux")]
pub fn simulate_paste() -> Result<()> {
    use std::process::Command;
    Command::new("xdotool")
        .args(["key", "ctrl+v"])
        .output()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn simulate_copy() -> Result<()> {
    use std::process::Command;
    Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "c" using command down"#,
        ])
        .output()?;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn simulate_paste() -> Result<()> {
    use std::process::Command;
    Command::new("osascript")
        .args([
            "-e",
            r#"tell application "System Events" to keystroke "v" using command down"#,
        ])
        .output()?;
    Ok(())
}
