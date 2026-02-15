use anyhow::{Context, Result};
use shared_child::SharedChild;
use std::collections::HashMap;
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

        eprintln!("[kanata] started (pid: {})", child.id());
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

/// kbd ファイルからキーバインド情報を抽出する。
///
/// 戻り値: `{ "apps": { "editor": "A", ... }, "search": { ... }, "folders": { ... } }`
pub fn parse_key_bindings() -> Result<HashMap<String, HashMap<String, String>>> {
    let kbd = KanataManager::kbd_path()?;
    let content = std::fs::read_to_string(&kbd)
        .with_context(|| format!("kbd ファイルの読み込みに失敗: {}", kbd.display()))?;

    let mut apps = HashMap::new();
    let mut search = HashMap::new();
    let mut folders = HashMap::new();

    // パターン: app-{key} (cmd muhenkan-switch switch-app --target {name})
    //           srch-{key} (cmd muhenkan-switch search --engine {name})
    //           fld-{key} (cmd muhenkan-switch open-folder --target {name})
    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("app-") {
            if let Some((key, name)) = parse_alias_line(rest, "switch-app --target") {
                apps.insert(name, key.to_uppercase());
            }
        } else if let Some(rest) = line.strip_prefix("srch-") {
            if let Some((key, name)) = parse_alias_line(rest, "search --engine") {
                search.insert(name, key.to_uppercase());
            }
        } else if let Some(rest) = line.strip_prefix("fld-") {
            if let Some((key, name)) = parse_alias_line(rest, "open-folder --target") {
                folders.insert(name, key.to_uppercase());
            }
        }
    }

    let mut result = HashMap::new();
    result.insert("apps".to_string(), apps);
    result.insert("search".to_string(), search);
    result.insert("folders".to_string(), folders);
    Ok(result)
}

/// エイリアス行からキーとターゲット名を抽出する。
/// 入力例: `a (cmd muhenkan-switch switch-app --target editor)`
/// 戻り値: `Some(("a", "editor"))`
fn parse_alias_line(rest: &str, command: &str) -> Option<(String, String)> {
    // rest = "a (cmd muhenkan-switch switch-app --target editor)"
    let key = rest.split_whitespace().next()?;
    let target = rest.split(command).nth(1)?
        .trim()
        .trim_end_matches(')')
        .trim();
    if target.is_empty() {
        return None;
    }
    Some((key.to_string(), target.to_string()))
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
