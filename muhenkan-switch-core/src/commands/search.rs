use anyhow::Result;
use arboard::Clipboard;

use crate::config::{self, Config};

pub fn run(engine: &str, config: &Config) -> Result<()> {
    // 検索エンジンのURLテンプレートを取得
    let url_template = config::get_search_url(&config.search, engine)?;

    // 元のクリップボードを保存
    let mut clipboard = Clipboard::new()?;
    let saved = clipboard.get_text().ok();

    // 選択テキストをクリップボードにコピー（Ctrl+C シミュレート）
    super::keys::simulate_copy()?;
    std::thread::sleep(std::time::Duration::from_millis(200));

    // クリップボードからテキスト取得
    let query = clipboard.get_text()?;

    if query.trim().is_empty() {
        // 復元してから返す
        if let Some(text) = saved {
            let _ = clipboard.set_text(text);
        }
        eprintln!("Warning: Clipboard is empty or contains no text.");
        return Ok(());
    }

    // URL組み立て＆ブラウザ起動
    let encoded = urlencoding::encode(query.trim());
    let url = url_template.replace("{query}", &encoded);
    webbrowser::open(&url)?;

    // クリップボードを復元
    if let Some(text) = saved {
        let _ = clipboard.set_text(text);
    }

    Ok(())
}
