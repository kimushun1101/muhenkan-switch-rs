use anyhow::Result;
use std::path::PathBuf;

use crate::config::{self, Config};

pub fn run(target: &str, config: &Config) -> Result<()> {
    let path_str = config::get_folder_path(&config.folders, target)?;

    if path_str.is_empty() {
        // パスが空の場合（ゴミ箱など）、OS ごとのデフォルト動作にフォールバック
        return open_platform_default(target);
    }

    // ~ をホームディレクトリに展開
    let path = expand_home(path_str);

    if !path.exists() {
        anyhow::bail!("Folder does not exist: {}", path.display());
    }

    open::that(&path)?;
    Ok(())
}

/// パスが空のフォルダエントリに対して、OS ごとのデフォルト動作でフォルダを開く。
fn open_platform_default(target: &str) -> Result<()> {
    match target {
        "trash" => {
            let path = imp::resolve_trash_path()?;
            open::that(&path)?;
            Ok(())
        }
        _ => anyhow::bail!(
            "Folder '{}' is not configured in config.toml (empty value)",
            target
        ),
    }
}

/// "~" または "~/" で始まるパスをホームディレクトリに展開する
fn expand_home(path_str: &str) -> PathBuf {
    if let Some(rest) = path_str.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    } else if path_str == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    PathBuf::from(path_str)
}

// ── Platform: Windows ──

#[cfg(target_os = "windows")]
mod imp {
    use anyhow::Result;
    use std::path::PathBuf;

    pub(super) fn resolve_trash_path() -> Result<PathBuf> {
        // Windows: shell:RecycleBinFolder を explorer.exe で開く
        // open::that では開けないため、直接 explorer を起動
        use std::process::Command;
        Command::new("explorer.exe")
            .arg("shell:RecycleBinFolder")
            .spawn()?;
        // ダミーパスを返す（呼び出し元では使われない）
        // TODO: open::that("shell:RecycleBinFolder") が動くか検証
        anyhow::bail!("Opened via explorer.exe")
    }
}

// ── Platform: Linux ──

#[cfg(target_os = "linux")]
mod imp {
    use anyhow::Result;
    use std::path::PathBuf;

    pub(super) fn resolve_trash_path() -> Result<PathBuf> {
        // FreeDesktop Trash Spec: ~/.local/share/Trash/files
        if let Some(data_local) = dirs::data_local_dir() {
            let trash = data_local.join("Trash").join("files");
            if trash.exists() {
                return Ok(trash);
            }
        }
        anyhow::bail!("Trash folder not found. Set [folders] trash path in config.toml")
    }
}

// ── Platform: macOS ──

#[cfg(target_os = "macos")]
mod imp {
    use anyhow::Result;
    use std::path::PathBuf;

    pub(super) fn resolve_trash_path() -> Result<PathBuf> {
        if let Some(home) = dirs::home_dir() {
            let trash = home.join(".Trash");
            if trash.exists() {
                return Ok(trash);
            }
        }
        anyhow::bail!("Trash folder not found. Set [folders] trash path in config.toml")
    }
}

// ── Tests ──

#[cfg(test)]
mod tests {
    use super::*;

    // ── expand_home ──

    #[test]
    fn expand_home_with_tilde_prefix() {
        let result = expand_home("~/Documents");
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home.join("Documents"));
    }

    #[test]
    fn expand_home_tilde_only() {
        let result = expand_home("~");
        let home = dirs::home_dir().unwrap();
        assert_eq!(result, home);
    }

    #[test]
    fn expand_home_absolute_path_unchanged() {
        let result = expand_home("/tmp/test");
        assert_eq!(result, PathBuf::from("/tmp/test"));
    }

    #[test]
    fn expand_home_relative_path_unchanged() {
        let result = expand_home("relative/path");
        assert_eq!(result, PathBuf::from("relative/path"));
    }

    #[test]
    fn expand_home_tilde_in_middle_unchanged() {
        // "foo/~/bar" のような場合は展開しない
        let result = expand_home("foo/~/bar");
        assert_eq!(result, PathBuf::from("foo/~/bar"));
    }

    // ── resolve_trash_path ──

    #[cfg(target_os = "linux")]
    #[test]
    fn resolve_trash_path_returns_freedesktop_path() {
        // Ubuntu にはゴミ箱フォルダがあるはず
        match imp::resolve_trash_path() {
            Ok(path) => {
                assert!(path.exists());
                assert!(
                    path.to_string_lossy().contains("Trash/files"),
                    "Expected path to contain 'Trash/files', got: {}",
                    path.display()
                );
            }
            Err(e) => {
                // Trash フォルダが存在しない環境（CI 等）ではエラーでも OK
                assert!(
                    e.to_string().contains("Trash folder not found"),
                    "Unexpected error: {}",
                    e
                );
            }
        }
    }

    // ── open_platform_default ──

    #[test]
    fn open_platform_default_unknown_target_errors() {
        let result = open_platform_default("nonexistent_folder");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("nonexistent_folder"),
            "Expected error to mention target name, got: {}",
            msg
        );
        assert!(
            msg.contains("empty value"),
            "Expected error to mention empty value, got: {}",
            msg
        );
    }

    // ── run (統合テスト) ──

    #[test]
    fn run_missing_folder_errors() {
        let config = Config {
            search: Default::default(),
            folders: Default::default(),
            apps: Default::default(),
            timestamp: Default::default(),
        };
        let result = run("nonexistent", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("not defined"));
    }

    #[test]
    fn run_nonexistent_path_errors() {
        use indexmap::IndexMap;
        use crate::config::FolderEntry;
        let mut folders = IndexMap::new();
        folders.insert(
            "test".to_string(),
            FolderEntry {
                key: Some("9".to_string()),
                path: "/tmp/__muhenkan_test_nonexistent_dir_12345__".to_string(),
            },
        );
        let config = Config {
            search: Default::default(),
            folders,
            apps: Default::default(),
            timestamp: Default::default(),
        };
        let result = run("test", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("does not exist"));
    }

    #[test]
    fn run_empty_path_unknown_target_errors() {
        use indexmap::IndexMap;
        use crate::config::FolderEntry;
        let mut folders = IndexMap::new();
        folders.insert(
            "unknown".to_string(),
            FolderEntry {
                key: Some("9".to_string()),
                path: "".to_string(),
            },
        );
        let config = Config {
            search: Default::default(),
            folders,
            apps: Default::default(),
            timestamp: Default::default(),
        };
        let result = run("unknown", &config);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("empty value"));
    }
}
