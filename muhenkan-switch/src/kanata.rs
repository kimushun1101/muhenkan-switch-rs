use anyhow::{Context, Result};
use shared_child::SharedChild;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager};

// ── Platform: Windows (Job Object) ──

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

// ── Platform: Linux (uinput support) ──

#[cfg(target_os = "linux")]
mod linux_support {
    use anyhow::{Context, Result};

    /// Wayland セッション判定
    pub fn is_wayland_session() -> bool {
        std::env::var("WAYLAND_DISPLAY").is_ok()
            || std::env::var("XDG_SESSION_TYPE")
                .map(|v| v == "wayland")
                .unwrap_or(false)
    }

    /// pkexec で uinput パーミッションを自動設定する
    pub fn setup_uinput_with_pkexec() -> Result<()> {
        let user = std::env::var("USER").unwrap_or_default();
        let script = format!(
            r#"
groupadd -f uinput
usermod -aG input {user}
usermod -aG uinput {user}
echo 'KERNEL=="uinput", MODE="0660", GROUP="uinput", OPTIONS+="static_node=uinput"' \
  > /etc/udev/rules.d/99-uinput.rules
udevadm control --reload-rules && udevadm trigger
"#
        );

        let status = std::process::Command::new("pkexec")
            .arg("bash")
            .arg("-c")
            .arg(&script)
            .status()
            .context("pkexec の実行に失敗しました")?;

        if status.success() {
            Ok(())
        } else {
            anyhow::bail!("設定がキャンセルされました")
        }
    }

    /// uinput パーミッション未設定の案内を stderr に表示
    pub fn print_uinput_guide() {
        use std::fs::OpenOptions;
        if OpenOptions::new()
            .write(true)
            .open("/dev/uinput")
            .is_ok()
        {
            return;
        }
        eprintln!();
        eprintln!("[kanata] uinput デバイスにアクセスできません。");
        eprintln!("[kanata] GUI からシステム設定ダイアログが表示されます。");
        eprintln!();
    }
}

// ── KanataManager ──

/// kanata バイナリ名
const fn kanata_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "kanata_cmd_allowed.exe"
    } else {
        "kanata_cmd_allowed"
    }
}

/// muhenkan-switch-core バイナリ名
const fn core_binary_name() -> &'static str {
    if cfg!(target_os = "windows") {
        "muhenkan-switch-core.exe"
    } else {
        "muhenkan-switch-core"
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
        let name = kanata_binary_name();

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
            "キー割当の起動に必要なファイルが見つかりません。\n\
             再インストールしてください。"
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
            "キー割当の設定ファイルが見つかりません。\n\
             再インストールしてください。"
        );
    }

    /// muhenkan-switch-core バイナリが存在するディレクトリを取得
    ///
    /// 探索順序:
    /// 1. exe と同じディレクトリ（インストール環境）
    /// 2. カレントディレクトリの bin/（開発環境: mise run build 後）
    /// 3. ワークスペースルートの bin/（開発環境）
    /// 4. target/debug/（開発環境: cargo build 直後）
    fn core_binary_dir() -> Result<PathBuf> {
        let name = core_binary_name();

        // 1. exe と同じディレクトリ
        if let Ok(exe_dir) = std::env::current_exe().map(|p| p.parent().unwrap().to_path_buf()) {
            if exe_dir.join(name).exists() {
                return Ok(exe_dir);
            }
        }

        // 2. カレントディレクトリの bin/
        if let Ok(cwd) = std::env::current_dir() {
            let bin_dir = cwd.join("bin");
            if bin_dir.join(name).exists() {
                return Ok(bin_dir);
            }
        }

        // 3. ワークスペースルートの bin/
        let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .map(|p| p.to_path_buf());
        if let Some(ref root) = workspace_root {
            let bin_dir = root.join("bin");
            if bin_dir.join(name).exists() {
                return Ok(bin_dir);
            }
        }

        // 4. target/debug/（開発環境）
        if let Some(ref root) = workspace_root {
            let debug_dir = root.join("target").join("debug");
            if debug_dir.join(name).exists() {
                return Ok(debug_dir);
            }
        }

        anyhow::bail!("キー割当の補助プログラムが見つかりません。\n再インストールしてください。");
    }

    pub fn start(&self) -> Result<()> {
        let mut guard = self.child.lock().unwrap();
        if let Some(ref child) = *guard {
            if child.try_wait().ok().flatten().is_none() {
                anyhow::bail!("キー割当は既に実行中です");
            }
        }

        let kanata = Self::kanata_path()?;
        let kbd = Self::kbd_path()?;

        eprintln!("[kanata] binary: {}", kanata.display());
        eprintln!("[kanata] config: {}", kbd.display());

        let mut cmd = std::process::Command::new(&kanata);
        cmd.arg("--cfg").arg(&kbd);

        // kanata の cmd 機能が muhenkan-switch-core を見つけられるよう PATH を設定
        if let Ok(core_dir) = Self::core_binary_dir() {
            let path = std::env::var("PATH").unwrap_or_default();
            let sep = if cfg!(windows) { ";" } else { ":" };
            cmd.env("PATH", format!("{}{}{}", core_dir.display(), sep, path));
            eprintln!("[kanata] core binary dir: {}", core_dir.display());
        }

        // Windows: GUI プロセスに非表示コンソールを割り当てる。
        // kanata がこのコンソールを継承し、(cmd ...) で起動される子プロセスも
        // 同じコンソールを継承するため、新しい可視ウィンドウが生成されない。
        // CREATE_NO_WINDOW を使うと kanata にコンソールが無くなり、
        // 子プロセスが新しい可視コンソールを作成してしまう。
        #[cfg(target_os = "windows")]
        {
            use windows::Win32::System::Console::{AllocConsole, GetConsoleWindow};
            use windows::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_HIDE};
            unsafe {
                if AllocConsole().is_ok() {
                    let console_hwnd = GetConsoleWindow();
                    if !console_hwnd.0.is_null() {
                        let _ = ShowWindow(console_hwnd, SW_HIDE);
                    }
                }
            }
        }

        let child = SharedChild::spawn(&mut cmd)
            .with_context(|| "キー割当の起動に失敗しました。\n再インストールしてください。".to_string())?;

        let pid = child.id();
        eprintln!("[kanata] started (pid: {})", pid);

        // 起動直後にクラッシュしていないか確認
        std::thread::sleep(Duration::from_millis(500));
        if let Ok(Some(_status)) = child.try_wait() {
            #[cfg(target_os = "linux")]
            linux_support::print_uinput_guide();

            anyhow::bail!("キー割当の起動に失敗しました。");
        }

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
            child.kill().context("キー割当の停止に失敗しました")?;
            child.wait().context("キー割当の終了待機に失敗しました")?;
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
    let need_uinput_dialog = if let Err(e) = manager.start() {
        eprintln!("[kanata] 自動開始に失敗: {:#}", e);

        #[cfg(target_os = "linux")]
        {
            use std::fs::OpenOptions;
            OpenOptions::new().write(true).open("/dev/uinput").is_err()
        }
        #[cfg(not(target_os = "linux"))]
        false
    } else {
        false
    };

    // Wayland 警告ダイアログ（イベントループ開始後に表示）
    #[cfg(target_os = "linux")]
    {
        if linux_support::is_wayland_session() {
            let handle = app.handle().clone();
            std::thread::spawn(move || {
                std::thread::sleep(Duration::from_secs(2));
                use tauri_plugin_dialog::{DialogExt, MessageDialogKind};

                handle
                    .dialog()
                    .message(
                        "Wayland セッションが検出されました。\n\n\
                         アプリ切り替え機能（無変換+A/W/E/S/D/F）は\n\
                         X11 セッションでのみ動作します。\n\n\
                         ログイン画面で「Ubuntu on Xorg」を選択して\n\
                         X11 セッションに切り替えてください。\n\n\
                         ※ カーソル移動・Web検索・フォルダ等の他の機能は\n\
                         Wayland でも動作します。",
                    )
                    .title("muhenkan-switch: Wayland の制約")
                    .kind(MessageDialogKind::Warning)
                    .blocking_show();
            });
        }
    }

    // uinput 設定ダイアログ（イベントループ開始後に表示）
    if need_uinput_dialog {
        let handle = app.handle().clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(1));
            use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};

            // 同意を求める
            let confirmed = handle
                .dialog()
                .message(
                    "キーボード機能を使用するにはシステム設定が必要です。\n\
                     「設定する」を押すと以下を自動で行います:\n\n\
                     ・キーボード制御用のグループを作成\n\
                     ・現在のユーザーをグループに追加\n\
                     ・デバイスのアクセス権限ルールを登録\n\n\
                     パスワードの入力が求められます。\n\
                     ※ この設定は初回のみ必要です。",
                )
                .title("muhenkan-switch: システム設定")
                .buttons(MessageDialogButtons::OkCancelCustom(
                    "設定する".into(),
                    "キャンセル".into(),
                ))
                .kind(MessageDialogKind::Info)
                .blocking_show();

            if !confirmed {
                return;
            }

            // pkexec で設定実行
            #[cfg(target_os = "linux")]
            match linux_support::setup_uinput_with_pkexec() {
                Ok(()) => {
                    handle
                        .dialog()
                        .message(
                            "設定が完了しました。\n\
                             反映するにはパソコンからログアウトし、\n\
                             再度ログインしてください。",
                        )
                        .title("muhenkan-switch: 設定完了")
                        .kind(MessageDialogKind::Info)
                        .blocking_show();
                }
                Err(e) => {
                    eprintln!("[kanata] uinput 設定に失敗: {:#}", e);
                }
            }
        });
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
