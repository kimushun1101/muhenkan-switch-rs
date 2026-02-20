use anyhow::Result;

pub fn simulate_copy() -> Result<()> {
    imp::simulate_copy()
}

pub fn simulate_paste() -> Result<()> {
    imp::simulate_paste()
}

// ── Platform: Windows ──

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use std::mem;
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VIRTUAL_KEY, VK_C,
        VK_CONTROL, VK_V,
    };

    pub(super) fn simulate_copy() -> Result<()> {
        send_ctrl_key(VK_C)
    }

    pub(super) fn simulate_paste() -> Result<()> {
        send_ctrl_key(VK_V)
    }

    /// Send Ctrl+<key> via Win32 SendInput.
    fn send_ctrl_key(vk: VIRTUAL_KEY) -> Result<()> {
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
}

// ── Platform: Linux ──

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::process::Command;

    pub(super) fn simulate_copy() -> Result<()> {
        Command::new("xdotool")
            .args(["key", "ctrl+c"])
            .output()?;
        Ok(())
    }

    pub(super) fn simulate_paste() -> Result<()> {
        Command::new("xdotool")
            .args(["key", "ctrl+v"])
            .output()?;
        Ok(())
    }
}

// ── Platform: macOS ──

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::process::Command;

    pub(super) fn simulate_copy() -> Result<()> {
        Command::new("osascript")
            .args([
                "-e",
                r#"tell application "System Events" to keystroke "c" using command down"#,
            ])
            .output()?;
        Ok(())
    }

    pub(super) fn simulate_paste() -> Result<()> {
        Command::new("osascript")
            .args([
                "-e",
                r#"tell application "System Events" to keystroke "v" using command down"#,
            ])
            .output()?;
        Ok(())
    }
}
