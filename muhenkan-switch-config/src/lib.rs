use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

// ── Types ──

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum AppEntry {
    Simple(String),
    Detailed {
        process: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        launch: Option<String>,
    },
}

impl AppEntry {
    pub fn process(&self) -> &str {
        match self {
            AppEntry::Simple(name) => name,
            AppEntry::Detailed { process, .. } => process,
        }
    }

    pub fn launch(&self) -> Option<&str> {
        match self {
            AppEntry::Simple(_) => None,
            AppEntry::Detailed { launch, .. } => launch.as_deref(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub search: HashMap<String, String>,
    #[serde(default)]
    pub folders: HashMap<String, String>,
    #[serde(default)]
    pub apps: HashMap<String, AppEntry>,
    #[serde(default)]
    pub timestamp: TimestampConfig,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
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

// ── Config path resolution ──

/// config.toml のパスを決定する。
/// 優先順位:
/// 1. 実行ファイルと同じディレクトリの config.toml
/// 2. カレントディレクトリの config.toml
/// 3. 見つからなければ None
pub fn config_path() -> Option<PathBuf> {
    // 実行ファイルと同じディレクトリ
    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            let path = dir.join("config.toml");
            if path.exists() {
                return Some(path);
            }
        }
    }

    // カレントディレクトリ
    let path = PathBuf::from("config.toml");
    if path.exists() {
        return Some(path);
    }

    None
}

/// 指定パスから config.toml を読み込む。
pub fn load_from(path: &std::path::Path) -> Result<Config> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read config file: {}", path.display()))?;
    let config: Config = toml::from_str(&content)
        .with_context(|| format!("Failed to parse config file: {}", path.display()))?;
    Ok(config)
}

/// config.toml を自動検出して読み込む。見つからなければデフォルト値。
pub fn load() -> Result<Config> {
    match config_path() {
        Some(path) => load_from(&path),
        None => {
            eprintln!("Warning: config.toml not found. Using default values.");
            Ok(default_config())
        }
    }
}

/// デフォルト設定を返す。
pub fn default_config() -> Config {
    Config {
        search: default_search_engines(),
        folders: HashMap::new(),
        apps: HashMap::new(),
        timestamp: TimestampConfig::default(),
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

// ── Save (comment-preserving) ──

/// config.toml にコメントを保持しつつ保存する。
/// 既存ファイルがあればコメントを保持、なければ新規作成。
pub fn save(path: &std::path::Path, config: &Config) -> Result<()> {
    use toml_edit::{DocumentMut, InlineTable, Item, Table, Value};

    // 既存ファイルがあればパースして構造を保持、なければ空ドキュメント
    let existing = if path.exists() {
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .parse::<DocumentMut>()
            .unwrap_or_default()
    } else {
        DocumentMut::new()
    };

    let mut doc = existing;

    // [search] セクション
    let search_table = doc
        .entry("search")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("search section is not a table")?;
    // Remove keys not in config
    let existing_keys: Vec<String> = search_table.iter().map(|(k, _)| k.to_string()).collect();
    for key in &existing_keys {
        if !config.search.contains_key(key) {
            search_table.remove(key);
        }
    }
    for (key, value) in &config.search {
        search_table[key] = toml_edit::value(value);
    }

    // [folders] セクション
    let folders_table = doc
        .entry("folders")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("folders section is not a table")?;
    let existing_keys: Vec<String> = folders_table.iter().map(|(k, _)| k.to_string()).collect();
    for key in &existing_keys {
        if !config.folders.contains_key(key) {
            folders_table.remove(key);
        }
    }
    for (key, value) in &config.folders {
        folders_table[key] = toml_edit::value(value);
    }

    // [apps] セクション
    let apps_table = doc
        .entry("apps")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("apps section is not a table")?;
    let existing_keys: Vec<String> = apps_table.iter().map(|(k, _)| k.to_string()).collect();
    for key in &existing_keys {
        if !config.apps.contains_key(key) {
            apps_table.remove(key);
        }
    }
    for (key, entry) in &config.apps {
        match entry {
            AppEntry::Simple(name) => {
                apps_table[key] = toml_edit::value(name);
            }
            AppEntry::Detailed { process, launch } => {
                let mut inline = InlineTable::new();
                inline.insert("process", Value::from(process.as_str()));
                if let Some(launch_cmd) = launch {
                    inline.insert("launch", Value::from(launch_cmd.as_str()));
                }
                apps_table[key] = toml_edit::value(inline);
            }
        }
    }

    // [timestamp] セクション
    let ts_table = doc
        .entry("timestamp")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("timestamp section is not a table")?;
    ts_table["format"] = toml_edit::value(&config.timestamp.format);
    ts_table["position"] = toml_edit::value(&config.timestamp.position);

    std::fs::write(path, doc.to_string())
        .with_context(|| format!("Failed to write config file: {}", path.display()))?;

    Ok(())
}

// ── Validation ──

/// 設定のバリデーション。エラーメッセージのリストを返す。
pub fn validate(config: &Config) -> Vec<String> {
    let mut errors = Vec::new();

    // timestamp format の検証
    if config.timestamp.format.is_empty() {
        errors.push("Timestamp format cannot be empty".to_string());
    }

    // timestamp position の検証
    if config.timestamp.position != "before" && config.timestamp.position != "after" {
        errors.push(format!(
            "Timestamp position must be \"before\" or \"after\", got \"{}\"",
            config.timestamp.position
        ));
    }

    // search URL テンプレートの検証
    for (key, url) in &config.search {
        if !url.contains("{query}") {
            errors.push(format!(
                "Search engine '{}' URL must contain {{query}} placeholder",
                key
            ));
        }
    }

    errors
}

// ── Helper ──

/// 設定から指定キーの値を取得するヘルパー
pub fn get_value<'a>(map: &'a HashMap<String, String>, key: &str, label: &str) -> Result<&'a str> {
    map.get(key)
        .map(|s| s.as_str())
        .ok_or_else(|| anyhow::anyhow!("{} '{}' is not defined in config.toml", label, key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = default_config();
        assert_eq!(config.timestamp.format, "%Y%m%d");
        assert_eq!(config.timestamp.position, "before");
        assert!(config.search.contains_key("google"));
    }

    #[test]
    fn test_validate_valid_config() {
        let config = default_config();
        let errors = validate(&config);
        assert!(errors.is_empty());
    }

    #[test]
    fn test_validate_invalid_position() {
        let mut config = default_config();
        config.timestamp.position = "middle".to_string();
        let errors = validate(&config);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("before"));
    }

    #[test]
    fn test_roundtrip_serialize() {
        let config = default_config();
        let toml_str = toml::to_string(&config).unwrap();
        let loaded: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(loaded.timestamp.format, config.timestamp.format);
    }
}
