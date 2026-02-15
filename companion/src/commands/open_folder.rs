use anyhow::Result;

use crate::config::{self, Config};

pub fn run(target: &str, config: &Config) -> Result<()> {
    let path_str = config::get_value(&config.folders, target, "Folder")?;

    if path_str.is_empty() {
        anyhow::bail!("Folder '{}' is not configured in config.toml (empty value)", target);
    }

    // ~ をホームディレクトリに展開
    let path = expand_home(path_str);

    if !path.exists() {
        anyhow::bail!("Folder does not exist: {}", path.display());
    }

    open::that(&path)?;
    Ok(())
}

/// "~" または "~/" で始まるパスをホームディレクトリに展開する
fn expand_home(path_str: &str) -> std::path::PathBuf {
    if let Some(rest) = path_str.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest);
        }
    } else if path_str == "~" {
        if let Some(home) = dirs::home_dir() {
            return home;
        }
    }
    std::path::PathBuf::from(path_str)
}
