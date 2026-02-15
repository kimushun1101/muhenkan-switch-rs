use anyhow::Result;
use arboard::Clipboard;
use chrono::Local;

use crate::config::Config;

pub fn run(action: &str, config: &Config) -> Result<()> {
    let now = Local::now();
    let timestamp = now.format(&config.timestamp.format).to_string();

    let mut clipboard = Clipboard::new()?;

    match action {
        "paste" => {
            // タイムスタンプをクリップボードにコピー
            clipboard.set_text(&timestamp)?;
            // NOTE: 実際の貼り付け（Ctrl+V 相当）は kanata 側で行う、
            // または外部コマンドで実現する必要がある
        }
        "copy" => {
            // 現在のクリップボードの内容にタイムスタンプを付加
            let current = clipboard.get_text().unwrap_or_default();
            let new_text = match config.timestamp.position.as_str() {
                "before" => format!("{}_{}", timestamp, current),
                "after" => format!("{}_{}", current, timestamp),
                _ => format!("{}_{}", timestamp, current),
            };
            clipboard.set_text(&new_text)?;
        }
        "cut" => {
            // タイムスタンプをクリップボードにコピー（cut と paste は
            // muhenkan-switch 側では同じ動作。切り取りは kanata 側で Ctrl+X を先に送る）
            clipboard.set_text(&timestamp)?;
        }
        _ => {
            anyhow::bail!("Unknown timestamp action: '{}'. Use paste, copy, or cut.", action);
        }
    }

    Ok(())
}
