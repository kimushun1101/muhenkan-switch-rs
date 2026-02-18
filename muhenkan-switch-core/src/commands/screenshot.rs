use anyhow::Result;

use crate::config::Config;

pub fn run(_config: &Config) -> Result<()> {
    imp::take_screenshot()
}

// ── Platform: Windows ──

#[cfg(target_os = "windows")]
mod imp {
    use super::*;
    use std::process::Command;

    pub(super) fn take_screenshot() -> Result<()> {
        // Windows: Snipping Tool を起動
        Command::new("snippingtool").spawn()?;
        Ok(())
    }
}

// ── Platform: Linux ──

#[cfg(target_os = "linux")]
mod imp {
    use super::*;
    use std::process::Command;

    pub(super) fn take_screenshot() -> Result<()> {
        // gnome-screenshot, flameshot, scrot のいずれかを試行
        let tools = [
            ("flameshot", vec!["gui"]),
            ("gnome-screenshot", vec!["-i"]),
            ("scrot", vec!["-s"]),
        ];

        for (tool, args) in &tools {
            if Command::new(tool).args(args).spawn().is_ok() {
                return Ok(());
            }
        }

        anyhow::bail!("No screenshot tool found. Install flameshot, gnome-screenshot, or scrot.");
    }
}

// ── Platform: macOS ──

#[cfg(target_os = "macos")]
mod imp {
    use super::*;
    use std::process::Command;

    pub(super) fn take_screenshot() -> Result<()> {
        // macOS: screencapture コマンド（未検証）
        Command::new("screencapture").args(["-i"]).spawn()?;
        Ok(())
    }
}
