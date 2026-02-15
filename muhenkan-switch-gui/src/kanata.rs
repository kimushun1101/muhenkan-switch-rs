use anyhow::{Context, Result};
use shared_child::SharedChild;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager};

/// Windows Job Object: GUI 終了時に子プロセス (kanata) を自動終了させる。
/// JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE により、ハンドルが閉じられると
/// 紐付けた全プロセスが OS によって強制終了される。
#[cfg(target_os = "windows")]
mod job_object {
    use windows::Win32::Foundation::{CloseHandle, HANDLE};
    use windows::Win32::System::JobObjects::{
        AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
        SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
        JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
    };
    use windows::Win32::System::Threading::{OpenProcess, PROCESS_SET_QUOTA, PROCESS_TERMINATE};

    pub struct JobObject(HANDLE);

    // SAFETY: HANDLE はスレッド間で安全に共有可能
    unsafe impl Send for JobObject {}
    unsafe impl Sync for JobObject {}

    impl JobObject {
        pub fn new() -> Option<Self> {
            unsafe {
                let job = CreateJobObjectW(None, None).ok()?;

                let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
                info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;

                SetInformationJobObject(
                    job,
                    JobObjectExtendedLimitInformation,
                    &info as *const _ as *const core::ffi::c_void,
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
                .ok()?;

                Some(Self(job))
            }
        }

        pub fn assign(&self, pid: u32) {
            unsafe {
                if let Ok(process) =
                    OpenProcess(PROCESS_SET_QUOTA | PROCESS_TERMINATE, false, pid)
                {
                    let _ = AssignProcessToJobObject(self.0, process);
                    let _ = CloseHandle(process);
                }
            }
        }
    }

    impl Drop for JobObject {
        fn drop(&mut self) {
            unsafe {
                let _ = CloseHandle(self.0);
            }
        }
    }
}

pub struct KanataManager {
    child: Arc<Mutex<Option<Arc<SharedChild>>>>,
    #[cfg(target_os = "windows")]
    job: Option<job_object::JobObject>,
}

impl KanataManager {
    pub fn new() -> Self {
        Self {
            child: Arc::new(Mutex::new(None)),
            #[cfg(target_os = "windows")]
            job: job_object::JobObject::new(),
        }
    }

    /// kanata バイナリのパスを取得
    ///
    /// 探索順序:
    /// 1. exe と同じディレクトリ（インストール環境）
    /// 2. カレントディレクトリ（開発環境: mise run gui 時）
    /// 3. ワークスペースルート（開発環境: CARGO_MANIFEST_DIR の親）
    fn kanata_path() -> Result<PathBuf> {
        #[cfg(target_os = "windows")]
        let name = "kanata_cmd_allowed.exe";
        #[cfg(not(target_os = "windows"))]
        let name = "kanata_cmd_allowed";

        // 1. exe と同じディレクトリ
        if let Ok(exe_dir) = std::env::current_exe().map(|p| p.parent().unwrap().to_path_buf()) {
            let path = exe_dir.join(name);
            if path.exists() {
                return Ok(path);
            }
        }

        // 2. カレントディレクトリ
        let cwd_path = PathBuf::from(name);
        if cwd_path.exists() {
            return Ok(std::env::current_dir()
                .unwrap_or_default()
                .join(name));
        }

        // 3. ワークスペースルート（開発環境）
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|p| p.to_path_buf());
        if let Some(ref root) = workspace_root {
            let path = root.join(name);
            if path.exists() {
                return Ok(path);
            }
        }

        anyhow::bail!(
            "kanata バイナリが見つかりません ({name})\n\
             プロジェクトルートに {name} を配置してください"
        );
    }

    /// kanata 設定ファイルのパスを取得
    ///
    /// 探索順序:
    /// 1. exe と同じディレクトリの muhenkan.kbd（インストール環境）
    /// 2. cwd の kanata/muhenkan.kbd（開発環境）
    /// 3. ワークスペースルートの kanata/muhenkan.kbd（開発環境）
    fn kbd_path() -> Result<PathBuf> {
        // 1. exe と同じディレクトリ
        if let Ok(exe_dir) = std::env::current_exe().map(|p| p.parent().unwrap().to_path_buf()) {
            let path = exe_dir.join("muhenkan.kbd");
            if path.exists() {
                return Ok(path);
            }
        }

        // 2. カレントディレクトリの kanata/ サブディレクトリ
        let cwd_path = PathBuf::from("kanata").join("muhenkan.kbd");
        if cwd_path.exists() {
            return Ok(std::env::current_dir()
                .unwrap_or_default()
                .join("kanata")
                .join("muhenkan.kbd"));
        }

        // 3. ワークスペースルート（開発環境）
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|p| p.to_path_buf());
        if let Some(ref root) = workspace_root {
            let path = root.join("kanata").join("muhenkan.kbd");
            if path.exists() {
                return Ok(path);
            }
        }

        anyhow::bail!(
            "kanata 設定ファイルが見つかりません (muhenkan.kbd)\n\
             kanata/muhenkan.kbd が存在するか確認してください"
        );
    }

    pub fn start(&self) -> Result<()> {
        let mut guard = self.child.lock().unwrap();
        if let Some(ref child) = *guard {
            if child.try_wait().ok().flatten().is_none() {
                anyhow::bail!("kanata は既に実行中です");
            }
        }

        let kanata = Self::kanata_path()?;
        let kbd = Self::kbd_path()?;

        eprintln!("[kanata] binary: {}", kanata.display());
        eprintln!("[kanata] config: {}", kbd.display());

        let mut cmd = std::process::Command::new(&kanata);
        cmd.arg("--cfg").arg(&kbd);

        #[cfg(target_os = "windows")]
        {
            use std::os::windows::process::CommandExt;
            const CREATE_NO_WINDOW: u32 = 0x08000000;
            cmd.creation_flags(CREATE_NO_WINDOW);
        }

        let child = SharedChild::spawn(&mut cmd)
            .with_context(|| format!(
                "kanata の起動に失敗しました\n\
                 バイナリ: {}\n\
                 設定: {}",
                kanata.display(), kbd.display()
            ))?;

        let pid = child.id();
        eprintln!("[kanata] started (pid: {})", pid);

        #[cfg(target_os = "windows")]
        if let Some(ref job) = self.job {
            job.assign(pid);
        }

        *guard = Some(Arc::new(child));

        Ok(())
    }

    pub fn stop(&self) -> Result<()> {
        let mut guard = self.child.lock().unwrap();
        if let Some(child) = guard.take() {
            child.kill().context("kanata プロセスの停止に失敗しました")?;
            child.wait().context("kanata プロセスの終了待機に失敗しました")?;
            eprintln!("[kanata] stopped");
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

/// アプリ起動時のセットアップ（kanata 自動開始 + 状態監視スレッド起動）
pub fn setup(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    // kanata を自動開始
    let manager = app.state::<KanataManager>();
    if let Err(e) = manager.start() {
        eprintln!("[kanata] 自動開始に失敗: {:#}", e);
    }

    let app_handle = app.handle().clone();
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
