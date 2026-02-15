use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Deserialize)]
pub struct Config {
    #[serde(default)]
    pub search: HashMap<String, String>,
    #[serde(default)]
    pub folders: HashMap<String, String>,
    #[serde(default)]
    pub apps: HashMap<String, String>,
    #[serde(default)]
    pub timestamp: TimestampConfig,
}

#[derive(Debug, Deserialize)]
pub struct TimestampConfig {
    #[serde(default = "default_format")]
    pub format: String,
    #[serde(default = "default_position")]
    pub position: String,
}

impl Default for TimestampConfig {
    fn default() -> Self {
        Self {
            format: default_format(),
            position: default_position(),
        }
    }
}

fn default_format() -> String {
    "%Y%m%d".to_string()
}

fn default_position() -> String {
    "before".to_string()
}

/// config.toml のパスを決定する。
/// 優先順位:
/// 1. 実行ファイルと同じディレクトリの config.toml
/// 2. 見つからなければデフォルト値を使用
fn config_path() -> Option<PathBuf> {
    // 実行ファイルと同じディレクトリ
    if let Ok(exe_path) = std::env::current_exe() {
        let dir = exe_path.parent()?;
        let path = dir.join("config.toml");
        if path.exists() {
            return Some(path);
        }
    }

    // カレントディレクトリ
    let path = PathBuf::from("config.toml");
    if path.exists() {
        return Some(path);
    }

    None
}

/// config.toml を読み込む。ファイルが見つからない場合はデフォルト値を使用。
pub fn load() -> Result<Config> {
    match config_path() {
        Some(path) => {
            let content = std::fs::read_to_string(&path)
                .with_context(|| format!("Failed to read config file: {}", path.display()))?;
            let config: Config = toml::from_str(&content)
                .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
            Ok(config)
        }
        None => {
            // config.toml が見つからない場合、デフォルト値で動作
            eprintln!("Warning: config.toml not found. Using default values.");
            Ok(Config {
                search: default_search_engines(),
                folders: HashMap::new(),
                apps: HashMap::new(),
                timestamp: TimestampConfig::default(),
            })
        }
    }
}

fn default_search_engines() -> HashMap<String, String> {
    let mut m = HashMap::new();
    m.insert(
        "google".to_string(),
        "https://www.google.com/search?q={query}".to_string(),
    );
    m
}

/// 設定から指定キーの値を取得するヘルパー
pub fn get_value<'a>(map: &'a HashMap<String, String>, key: &str, label: &str) -> Result<&'a str> {
    map.get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow::anyhow!("{} '{}' is not defined in config.toml", label, key))
}
