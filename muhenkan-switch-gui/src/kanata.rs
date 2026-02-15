use anyhow::{Context, Result};
use shared_child::SharedChild;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager};

pub struct KanataManager {
    child: Arc<Mutex<Option<Arc<SharedChild>>>>,
}

impl KanataManager {
    pub fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
        }
    }

    /// kanata バイナリのパスを取得
    fn kanata_path() -> Result<PathBuf> {
        let exe_dir = std::env::current_exe()
            .context("Cannot determine exe path")?
            .parent()
            .context("Cannot determine exe directory")?
            .to_path_buf();

        #[cfg(target_os = "windows")]
        let name = "kanata_cmd_allowed.exe";
        #[cfg(not(target_os = "windows"))]
        let name = "kanata_cmd_allowed";

        let path = exe_dir.join(name);
        if !path.exists() {
            // Also check current directory
            let cwd_path = PathBuf::from(name);
            if cwd_path.exists() {
                return Ok(cwd_path);
            }
            anyhow::bail!("kanata binary not found: {}", path.display());
        }
        Ok(path)
    }

    /// kanata 設定ファイルのパスを取得
    fn kbd_path() -> Result<PathBuf> {
        let exe_dir = std::env::current_exe()
            .context("Cannot determine exe path")?
            .parent()
            .context("Cannot determine exe directory")?
            .to_path_buf();

        let path = exe_dir.join("muhenkan.kbd");
        if !path.exists() {
            // Also check kanata/ subdirectory
            let sub_path = exe_dir.join("kanata").join("muhenkan.kbd");
            if sub_path.exists() {
                return Ok(sub_path);
            }
            // Check current directory
            let cwd_path = PathBuf::from("kanata").join("muhenkan.kbd");
            if cwd_path.exists() {
                return Ok(cwd_path);
            }
            anyhow::bail!("kanata config not found: {}", path.display());
        }
        Ok(path)
    }

    pub fn start(&self) -> Result<()> {
        let mut guard = self.child.lock().unwrap();
        if let Some(ref child) = *guard {
            if child.try_wait().ok().flatten().is_none() {
                anyhow::bail!("kanata is already running");
            }
        }

        let kanata = Self::kanata_path()?;
        let kbd = Self::kbd_path()?;

        let mut cmd = std::process::Command::new(&kanata);
        cmd.arg("--cfg").arg(&kbd);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = SharedChild::spawn(&mut cmd)
            .with_context(|| format!("Failed to start kanata: {}", kanata.display()))?;
        *guard = Some(Arc::new(child));

        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let mut guard = self.child.lock().unwrap();
        if let Some(child) = guard.take() {
            child.kill().context("Failed to kill kanata")?;
            child.wait().context("Failed to wait for kanata")?;
        }
        Ok(())
    }

    pub fn restart(&self) -> Result<()> {
        self.stop()?;
        std::thread::sleep(Duration::from_millis(500));
        self.start()
    }

    pub fn status(&self) -> (bool, Option<u32>) {
        let guard = self.child.lock().unwrap();
        match &*guard {
            Some(child) => {
                let running = child.try_wait().ok().flatten().is_none();
                let pid = child.id();
                (running, Some(pid))
            }
            None => (false, None),
        }
    }
}

/// アプリ起動時のセットアップ（状態監視スレッド起動）
pub fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let app_handle = app.handle().clone();
    let manager = app.state::<KanataManager>();
    let child_ref = Arc::clone(&manager.child);

    // 状態監視スレッド
    std::thread::spawn(move || {
        let mut last_running = false;
        loop {
            std::thread::sleep(Duration::from_secs(2));

            let running = {
                let guard = child_ref.lock().unwrap();
                match &*guard {
                    Some(child) => child.try_wait().ok().flatten().is_none(),
                    None => false,
                }
            };

            if running != last_running {
                last_running = running;
                let _ = app_handle.emit("kanata-status-changed", running);
            }
        }
    });

    Ok(())
}
