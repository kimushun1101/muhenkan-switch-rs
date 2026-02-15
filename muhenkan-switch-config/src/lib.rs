use anyhow::{Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

// ── Dispatch keys ──

/// kbd ファイルでディスパッチに割り当てられている物理キーの一覧。
pub const DISPATCH_KEYS: &[&str] = &[
    "1", "2", "3", "4", "5",
    "q", "r", "t", "g",
    "a", "w", "e", "s", "d", "f",
];

// ── Types ──

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum SearchEntry {
    Simple(String),
    Detailed {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        url: String,
    },
}

impl SearchEntry {
    pub fn url(&self) -> &str {
        match self {
            SearchEntry::Simple(url) => url,
            SearchEntry::Detailed { url, .. } => url,
        }
    }

    pub fn dispatch_key(&self) -> Option<&str> {
        match self {
            SearchEntry::Simple(_) => None,
            SearchEntry::Detailed { key, .. } => key.as_deref(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum FolderEntry {
    Simple(String),
    Detailed {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        path: String,
    },
}

impl FolderEntry {
    pub fn path(&self) -> &str {
        match self {
            FolderEntry::Simple(path) => path,
            FolderEntry::Detailed { path, .. } => path,
        }
    }

    pub fn dispatch_key(&self) -> Option<&str> {
        match self {
            FolderEntry::Simple(_) => None,
            FolderEntry::Detailed { key, .. } => key.as_deref(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
#[serde(untagged)]
pub enum AppEntry {
    Simple(String),
    Detailed {
        #[serde(skip_serializing_if = "Option::is_none")]
        key: Option<String>,
        process: String,
        #[serde(skip_serializing_if = "Option::is_none", alias = "launch")]
        command: Option<String>,
    },
}

impl AppEntry {
    pub fn process(&self) -> &str {
        match self {
            AppEntry::Simple(name) => name,
            AppEntry::Detailed { process, .. } => process,
        }
    }

    /// 起動コマンドを返す。未設定の場合はプロセス名をフォールバックとして使う。
    pub fn command(&self) -> Option<&str> {
        match self {
            AppEntry::Simple(name) => Some(name.as_str()),
            AppEntry::Detailed { process, command, .. } => {
                Some(command.as_deref().unwrap_or(process.as_str()))
            }
        }
    }

    pub fn dispatch_key(&self) -> Option<&str> {
        match self {
            AppEntry::Simple(_) => None,
            AppEntry::Detailed { key, .. } => key.as_deref(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum DispatchAction {
    Search { engine: String },
    OpenFolder { target: String },
    SwitchApp { target: String },
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(default)]
    pub search: IndexMap<String, SearchEntry>,
    #[serde(default)]
    pub folders: IndexMap<String, FolderEntry>,
    #[serde(default)]
    pub apps: IndexMap<String, AppEntry>,
    #[serde(default)]
    pub timestamp: TimestampConfig,
}

impl Config {
    /// ディスパッチキーに対応するアクションを検索する。
    pub fn dispatch_lookup(&self, key: &str) -> Option<DispatchAction> {
        for (name, entry) in &self.search {
            if entry.dispatch_key() == Some(key) {
                return Some(DispatchAction::Search {
                    engine: name.clone(),
                });
            }
        }
        for (name, entry) in &self.folders {
            if entry.dispatch_key() == Some(key) {
                return Some(DispatchAction::OpenFolder {
                    target: name.clone(),
                });
            }
        }
        for (name, entry) in &self.apps {
            if entry.dispatch_key() == Some(key) {
                return Some(DispatchAction::SwitchApp {
                    target: name.clone(),
                });
            }
        }
        None
    }
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
        folders: IndexMap::new(),
        apps: IndexMap::new(),
        timestamp: TimestampConfig::default(),
    }
}

fn default_search_engines() -> IndexMap<String, SearchEntry> {
    let mut m = IndexMap::new();
    m.insert(
        "google".to_string(),
        SearchEntry::Simple("https://www.google.com/search?q={query}".to_string()),
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

    // [search] セクション — 順序を保持するため全削除→再挿入
    let search_table = doc
        .entry("search")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("search section is not a table")?;
    search_table.clear();
    for (name, entry) in &config.search {
        match entry {
            SearchEntry::Simple(url) => {
                search_table[name] = toml_edit::value(url);
            }
            SearchEntry::Detailed { key, url } => {
                let mut inline = InlineTable::new();
                if let Some(dk) = key {
                    inline.insert("key", Value::from(dk.as_str()));
                }
                inline.insert("url", Value::from(url.as_str()));
                search_table[name] = toml_edit::value(inline);
            }
        }
    }

    // [folders] セクション
    let folders_table = doc
        .entry("folders")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("folders section is not a table")?;
    folders_table.clear();
    for (name, entry) in &config.folders {
        match entry {
            FolderEntry::Simple(path) => {
                folders_table[name] = toml_edit::value(path);
            }
            FolderEntry::Detailed { key, path } => {
                let mut inline = InlineTable::new();
                if let Some(dk) = key {
                    inline.insert("key", Value::from(dk.as_str()));
                }
                inline.insert("path", Value::from(path.as_str()));
                folders_table[name] = toml_edit::value(inline);
            }
        }
    }

    // [apps] セクション
    let apps_table = doc
        .entry("apps")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("apps section is not a table")?;
    apps_table.clear();
    for (name, entry) in &config.apps {
        match entry {
            AppEntry::Simple(process_name) => {
                apps_table[name] = toml_edit::value(process_name);
            }
            AppEntry::Detailed {
                key,
                process,
                command,
            } => {
                let mut inline = InlineTable::new();
                if let Some(dk) = key {
                    inline.insert("key", Value::from(dk.as_str()));
                }
                inline.insert("process", Value::from(process.as_str()));
                if let Some(cmd) = command {
                    inline.insert("command", Value::from(cmd.as_str()));
                }
                apps_table[name] = toml_edit::value(inline);
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
    for (name, entry) in &config.search {
        if !entry.url().contains("{query}") {
            errors.push(format!(
                "Search engine '{}' URL must contain {{query}} placeholder",
                name
            ));
        }
    }

    // ディスパッチキーの重複チェック
    let mut used_keys: IndexMap<String, String> = IndexMap::new();
    for (name, entry) in &config.search {
        if let Some(k) = entry.dispatch_key() {
            let label = format!("search/{}", name);
            if let Some(prev) = used_keys.get(k) {
                errors.push(format!(
                    "Dispatch key '{}' is used by both '{}' and '{}'",
                    k, prev, label
                ));
            } else {
                used_keys.insert(k.to_string(), label);
            }
        }
    }
    for (name, entry) in &config.folders {
        if let Some(k) = entry.dispatch_key() {
            let label = format!("folders/{}", name);
            if let Some(prev) = used_keys.get(k) {
                errors.push(format!(
                    "Dispatch key '{}' is used by both '{}' and '{}'",
                    k, prev, label
                ));
            } else {
                used_keys.insert(k.to_string(), label);
            }
        }
    }
    for (name, entry) in &config.apps {
        if let Some(k) = entry.dispatch_key() {
            let label = format!("apps/{}", name);
            if let Some(prev) = used_keys.get(k) {
                errors.push(format!(
                    "Dispatch key '{}' is used by both '{}' and '{}'",
                    k, prev, label
                ));
            } else {
                used_keys.insert(k.to_string(), label);
            }
        }
    }

    errors
}

// ── Helpers ──

/// 検索エンジンの URL テンプレートを取得する。
pub fn get_search_url<'a>(
    search: &'a IndexMap<String, SearchEntry>,
    engine: &str,
) -> Result<&'a str> {
    search
        .get(engine)
        .map(|e| e.url())
        .ok_or_else(|| anyhow::anyhow!("Search engine '{}' is not defined in config.toml", engine))
}

/// フォルダのパスを取得する。
pub fn get_folder_path<'a>(
    folders: &'a IndexMap<String, FolderEntry>,
    target: &str,
) -> Result<&'a str> {
    folders
        .get(target)
        .map(|e| e.path())
        .ok_or_else(|| anyhow::anyhow!("Folder '{}' is not defined in config.toml", target))
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

    #[test]
    fn test_parse_old_format() {
        let toml_str = r#"
            [search]
            google = "https://www.google.com/search?q={query}"

            [folders]
            documents = "~/Documents"

            [apps]
            editor = {process = "Code", command = "code"}
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.search["google"].url(), "https://www.google.com/search?q={query}");
        assert!(config.search["google"].dispatch_key().is_none());
        assert_eq!(config.folders["documents"].path(), "~/Documents");
        assert!(config.folders["documents"].dispatch_key().is_none());
        assert_eq!(config.apps["editor"].process(), "Code");
        assert!(config.apps["editor"].dispatch_key().is_none());
    }

    #[test]
    fn test_parse_new_format_with_keys() {
        let toml_str = r#"
            [search]
            google = {key = "g", url = "https://www.google.com/search?q={query}"}

            [folders]
            documents = {key = "1", path = "~/Documents"}

            [apps]
            editor = {key = "a", process = "Code", command = "code"}
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.search["google"].url(), "https://www.google.com/search?q={query}");
        assert_eq!(config.search["google"].dispatch_key(), Some("g"));
        assert_eq!(config.folders["documents"].path(), "~/Documents");
        assert_eq!(config.folders["documents"].dispatch_key(), Some("1"));
        assert_eq!(config.apps["editor"].process(), "Code");
        assert_eq!(config.apps["editor"].dispatch_key(), Some("a"));
    }

    #[test]
    fn test_dispatch_lookup() {
        let toml_str = r#"
            [search]
            google = {key = "g", url = "https://www.google.com/search?q={query}"}

            [folders]
            documents = {key = "1", path = "~/Documents"}

            [apps]
            editor = {key = "a", process = "Code", command = "code"}
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();

        match config.dispatch_lookup("g") {
            Some(DispatchAction::Search { engine }) => assert_eq!(engine, "google"),
            other => panic!("Expected Search, got {:?}", other),
        }
        match config.dispatch_lookup("1") {
            Some(DispatchAction::OpenFolder { target }) => assert_eq!(target, "documents"),
            other => panic!("Expected OpenFolder, got {:?}", other),
        }
        match config.dispatch_lookup("a") {
            Some(DispatchAction::SwitchApp { target }) => assert_eq!(target, "editor"),
            other => panic!("Expected SwitchApp, got {:?}", other),
        }
        assert!(config.dispatch_lookup("z").is_none());
    }

    #[test]
    fn test_validate_duplicate_keys() {
        let toml_str = r#"
            [search]
            google = {key = "a", url = "https://www.google.com/search?q={query}"}

            [apps]
            editor = {key = "a", process = "Code", command = "code"}
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate(&config);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Dispatch key 'a'"));
    }
}
