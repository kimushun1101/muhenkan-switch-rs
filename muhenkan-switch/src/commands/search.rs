use anyhow::Result;
use arboard::Clipboard;

use crate::config::{self, Config};

pub fn run(engine: &str, config: &Config) -> Result<()> {
    // 検索エンジンのURLテンプレートを取得
    let url_template = config::get_value(&config.search, engine, "Search engine")?;

    // クリップボードからテキスト取得
    let mut clipboard = Clipboard::new()?;
    let query = clipboard.get_text()?;

    if query.trim().is_empty() {
        eprintln!("Warning: Clipboard is empty or contains no text.");
        return Ok(());
    }

    // URL組み立て＆ブラウザ起動
    let encoded = urlencoding::encode(query.trim());
    let url = url_template.replace("{query}", &encoded);
    webbrowser::open(&url)?;

    Ok(())
}
