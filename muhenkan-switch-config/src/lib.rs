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
pub struct SearchEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub url: String,
}

impl SearchEntry {
    pub fn url(&self) -> &str {
        &self.url
    }

    pub fn dispatch_key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct FolderEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub path: String,
}

impl FolderEntry {
    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn dispatch_key(&self) -> Option<&str> {
        self.key.as_deref()
    }
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct AppEntry {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    pub process: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
}

impl AppEntry {
    pub fn process(&self) -> &str {
        &self.process
    }

    /// 起動コマンドを返す。未設定の場合はプロセス名をフォールバックとして使う。
    pub fn command(&self) -> Option<&str> {
        Some(self.command.as_deref().unwrap_or(&self.process))
    }

    pub fn dispatch_key(&self) -> Option<&str> {
        self.key.as_deref()
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
        SearchEntry {
            key: None,
            url: "https://www.google.com/search?q={query}".to_string(),
        },
    );
    m
}

// ── Save (comment-preserving) ──

/// config.toml にコメントを保持しつつ保存する。
/// 既存ファイルがあればコメントを保持、なければ新規作成。
/// エントリはディスパッチキー順でソートされる（キーなしは末尾、名前順）。
pub fn save(path: &std::path::Path, config: &Config) -> Result<()> {
    use toml_edit::{DocumentMut, InlineTable, Item, Table, Value};

    // ソート用ヘルパー: dispatch key あり → key 昇順、なし → 名前昇順で末尾
    fn sort_key<'a>(dispatch_key: Option<&'a str>, name: &'a str) -> (u8, &'a str) {
        match dispatch_key {
            Some(k) => (0, k),
            None => (1, name),
        }
    }

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
    search_table.clear();
    let mut search_entries: Vec<_> = config.search.iter().collect();
    search_entries.sort_by(|(na, a), (nb, b)| {
        sort_key(a.dispatch_key(), na).cmp(&sort_key(b.dispatch_key(), nb))
    });
    for (name, entry) in search_entries {
        let mut inline = InlineTable::new();
        if let Some(dk) = &entry.key {
            inline.insert("key", Value::from(dk.as_str()));
        }
        inline.insert("url", Value::from(entry.url.as_str()));
        search_table[name] = toml_edit::value(inline);
    }

    // [folders] セクション
    let folders_table = doc
        .entry("folders")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("folders section is not a table")?;
    folders_table.clear();
    let mut folder_entries: Vec<_> = config.folders.iter().collect();
    folder_entries.sort_by(|(na, a), (nb, b)| {
        sort_key(a.dispatch_key(), na).cmp(&sort_key(b.dispatch_key(), nb))
    });
    for (name, entry) in folder_entries {
        let mut inline = InlineTable::new();
        if let Some(dk) = &entry.key {
            inline.insert("key", Value::from(dk.as_str()));
        }
        inline.insert("path", Value::from(entry.path.as_str()));
        folders_table[name] = toml_edit::value(inline);
    }

    // [apps] セクション
    let apps_table = doc
        .entry("apps")
        .or_insert_with(|| Item::Table(Table::new()))
        .as_table_mut()
        .context("apps section is not a table")?;
    apps_table.clear();
    let mut app_entries: Vec<_> = config.apps.iter().collect();
    app_entries.sort_by(|(na, a), (nb, b)| {
        sort_key(a.dispatch_key(), na).cmp(&sort_key(b.dispatch_key(), nb))
    });
    for (name, entry) in app_entries {
        let mut inline = InlineTable::new();
        if let Some(dk) = &entry.key {
            inline.insert("key", Value::from(dk.as_str()));
        }
        inline.insert("process", Value::from(entry.process.as_str()));
        if let Some(cmd) = &entry.command {
            inline.insert("command", Value::from(cmd.as_str()));
        }
        apps_table[name] = toml_edit::value(inline);
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

    // ── A. パース (追加分) ──

    #[test]
    fn test_parse_without_key() {
        let toml_str = r#"
            [search]
            google = {url = "https://www.google.com/search?q={query}"}

            [folders]
            documents = {path = "~/Documents"}

            [apps]
            editor = {process = "Code", command = "code"}
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.search["google"].dispatch_key().is_none());
        assert_eq!(config.search["google"].url(), "https://www.google.com/search?q={query}");
        assert!(config.folders["documents"].dispatch_key().is_none());
        assert!(config.apps["editor"].dispatch_key().is_none());
    }

    #[test]
    fn test_parse_empty_sections() {
        let toml_str = r#"
            [search]
            [folders]
            [apps]
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.search.is_empty());
        assert!(config.folders.is_empty());
        assert!(config.apps.is_empty());
    }

    #[test]
    fn test_parse_missing_sections() {
        let toml_str = r#"
            [timestamp]
            format = "%Y%m%d"
            position = "before"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert!(config.search.is_empty());
        assert!(config.folders.is_empty());
        assert!(config.apps.is_empty());
    }

    // ── B. ディスパッチ検索 (追加分) ──

    #[test]
    fn test_dispatch_lookup_priority() {
        // search takes priority over apps when both have the same key
        let toml_str = r#"
            [search]
            google = {key = "a", url = "https://www.google.com/search?q={query}"}

            [apps]
            editor = {key = "a", process = "Code", command = "code"}
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        match config.dispatch_lookup("a") {
            Some(DispatchAction::Search { engine }) => assert_eq!(engine, "google"),
            other => panic!("Expected Search (priority over Apps), got {:?}", other),
        }
    }

    // ── C. バリデーション (追加分) ──

    #[test]
    fn test_validate_empty_format() {
        let mut config = default_config();
        config.timestamp.format = String::new();
        let errors = validate(&config);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("format"));
    }

    #[test]
    fn test_validate_missing_query_placeholder() {
        let toml_str = r#"
            [search]
            bad = {key = "g", url = "https://example.com/search"}
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate(&config);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("{query}"));
    }

    #[test]
    fn test_validate_duplicate_keys_same_section() {
        let toml_str = r#"
            [search]
            google = {key = "g", url = "https://www.google.com/search?q={query}"}
            ejje = {key = "g", url = "https://ejje.weblio.jp/content/{query}"}
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();
        let errors = validate(&config);
        assert_eq!(errors.len(), 1);
        assert!(errors[0].contains("Dispatch key 'g'"));
        assert!(errors[0].contains("search/google"));
        assert!(errors[0].contains("search/ejje"));
    }

    #[test]
    fn test_validate_multiple_errors() {
        let mut config = default_config();
        config.timestamp.format = String::new();
        config.timestamp.position = "middle".to_string();
        config.search.insert(
            "bad".to_string(),
            SearchEntry {
                key: Some("g".to_string()),
                url: "https://example.com/no-placeholder".to_string(),
            },
        );
        let errors = validate(&config);
        assert!(errors.len() >= 3, "Expected at least 3 errors, got: {:?}", errors);
    }

    #[test]
    fn test_validate_all_keys_assigned() {
        let mut config = Config {
            search: IndexMap::new(),
            folders: IndexMap::new(),
            apps: IndexMap::new(),
            timestamp: TimestampConfig::default(),
        };
        // Assign all 15 dispatch keys across sections
        let keys = DISPATCH_KEYS;
        for (i, key) in keys.iter().enumerate() {
            let name = format!("entry_{}", i);
            if i < 5 {
                config.search.insert(
                    name,
                    SearchEntry {
                        key: Some(key.to_string()),
                        url: format!("https://example.com/{}?q={{query}}", key),
                    },
                );
            } else if i < 10 {
                config.folders.insert(
                    name,
                    FolderEntry {
                        key: Some(key.to_string()),
                        path: format!("~/{}", key),
                    },
                );
            } else {
                config.apps.insert(
                    name,
                    AppEntry {
                        key: Some(key.to_string()),
                        process: format!("app_{}", key),
                        command: None,
                    },
                );
            }
        }
        let errors = validate(&config);
        assert!(errors.is_empty(), "Expected no errors, got: {:?}", errors);
    }

    // ── D. Save/Load ラウンドトリップ (追加分) ──

    #[test]
    fn test_roundtrip_save_load_detailed() {
        let toml_str = r#"
            [search]
            google = {key = "g", url = "https://www.google.com/search?q={query}"}

            [folders]
            documents = {key = "1", path = "~/Documents"}

            [apps]
            editor = {key = "a", process = "Code", command = "code"}

            [timestamp]
            format = "%Y%m%d"
            position = "before"
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();

        let dir = std::env::temp_dir().join("muhenkan_test_roundtrip_detailed");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        save(&path, &config).unwrap();
        let loaded = load_from(&path).unwrap();

        // Verify search
        assert_eq!(loaded.search["google"].url(), "https://www.google.com/search?q={query}");
        assert_eq!(loaded.search["google"].dispatch_key(), Some("g"));

        // Verify folders
        assert_eq!(loaded.folders["documents"].path(), "~/Documents");
        assert_eq!(loaded.folders["documents"].dispatch_key(), Some("1"));

        // Verify apps
        assert_eq!(loaded.apps["editor"].process(), "Code");
        assert_eq!(loaded.apps["editor"].command(), Some("code"));
        assert_eq!(loaded.apps["editor"].dispatch_key(), Some("a"));

        // Verify timestamp
        assert_eq!(loaded.timestamp.format, "%Y%m%d");
        assert_eq!(loaded.timestamp.position, "before");

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_save_creates_file() {
        let dir = std::env::temp_dir().join("muhenkan_test_save_creates");
        // Ensure clean state
        std::fs::remove_dir_all(&dir).ok();
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        assert!(!path.exists());

        let config = default_config();
        save(&path, &config).unwrap();
        assert!(path.exists());

        let loaded = load_from(&path).unwrap();
        assert_eq!(loaded.timestamp.format, "%Y%m%d");

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn test_save_sorts_by_dispatch_key() {
        // 名前のアルファベット順とディスパッチキー順が異なるデータ
        let toml_str = r#"
            [search]
            gamma = {key = "g", url = "https://gamma.com/?q={query}"}
            alpha = {key = "t", url = "https://alpha.com/?q={query}"}
            beta = {key = "r", url = "https://beta.com/?q={query}"}
        "#;
        let config: Config = toml::from_str(toml_str).unwrap();

        let dir = std::env::temp_dir().join("muhenkan_test_sort_key");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");

        save(&path, &config).unwrap();
        let loaded = load_from(&path).unwrap();

        // dispatch key 順: g < r < t → gamma, beta, alpha
        let names: Vec<&String> = loaded.search.keys().collect();
        assert_eq!(names, vec!["gamma", "beta", "alpha"]);

        // Cleanup
        std::fs::remove_dir_all(&dir).ok();
    }

    // ── E. ヘルパー関数 ──

    #[test]
    fn test_get_search_url_found() {
        let mut search = IndexMap::new();
        search.insert(
            "google".to_string(),
            SearchEntry {
                key: Some("g".to_string()),
                url: "https://www.google.com/search?q={query}".to_string(),
            },
        );
        let url = get_search_url(&search, "google").unwrap();
        assert_eq!(url, "https://www.google.com/search?q={query}");
    }

    #[test]
    fn test_get_search_url_not_found() {
        let search = IndexMap::new();
        let result = get_search_url(&search, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }

    #[test]
    fn test_get_folder_path_found() {
        let mut folders = IndexMap::new();
        folders.insert(
            "docs".to_string(),
            FolderEntry {
                key: Some("1".to_string()),
                path: "~/Documents".to_string(),
            },
        );
        let path = get_folder_path(&folders, "docs").unwrap();
        assert_eq!(path, "~/Documents");
    }

    #[test]
    fn test_get_folder_path_not_found() {
        let folders = IndexMap::new();
        let result = get_folder_path(&folders, "nonexistent");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("nonexistent"));
    }

    #[test]
    fn test_app_entry_command_fallback() {
        let entry = AppEntry {
            key: Some("a".to_string()),
            process: "Code".to_string(),
            command: None,
        };
        // command() falls back to process name when command is None
        assert_eq!(entry.command(), Some("Code"));
        assert_eq!(entry.process(), "Code");

        // When command is explicitly set, it takes priority
        let entry2 = AppEntry {
            key: Some("a".to_string()),
            process: "Code".to_string(),
            command: Some("code".to_string()),
        };
        assert_eq!(entry2.command(), Some("code"));
    }
}
