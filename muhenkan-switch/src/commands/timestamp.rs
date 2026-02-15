use anyhow::Result;
use arboard::Clipboard;
use chrono::Local;

use crate::config::Config;

/// "copy" アクション用: timestamp と current を position に応じて結合
fn compose_copy_text(timestamp: &str, current: &str, position: &str) -> String {
    match position {
        "after" => format!("{}_{}", current, timestamp),
        // "before" およびその他は before 扱い
        _ => format!("{}_{}", timestamp, current),
    }
}

/// アクションに応じてクリップボードに書き込むテキストを決定
fn resolve_timestamp_text(
    action: &str,
    timestamp: &str,
    clipboard_text: &str,
    position: &str,
) -> Result<String> {
    match action {
        "paste" | "cut" => Ok(timestamp.to_string()),
        "copy" => Ok(compose_copy_text(timestamp, clipboard_text, position)),
        _ => anyhow::bail!(
            "Unknown timestamp action: '{}'. Use paste, copy, or cut.",
            action
        ),
    }
}

pub fn run(action: &str, config: &Config) -> Result<()> {
    let timestamp = Local::now().format(&config.timestamp.format).to_string();
    let mut clipboard = Clipboard::new()?;
    let clipboard_text = if action == "copy" {
        clipboard.get_text().unwrap_or_default()
    } else {
        String::new()
    };
    let text = resolve_timestamp_text(
        action,
        &timestamp,
        &clipboard_text,
        &config.timestamp.position,
    )?;
    clipboard.set_text(&text)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- compose_copy_text ---

    #[test]
    fn test_compose_copy_text_before() {
        assert_eq!(compose_copy_text("TS", "current", "before"), "TS_current");
    }

    #[test]
    fn test_compose_copy_text_after() {
        assert_eq!(compose_copy_text("TS", "current", "after"), "current_TS");
    }

    #[test]
    fn test_compose_copy_text_unknown_position_defaults_to_before() {
        assert_eq!(compose_copy_text("TS", "current", "middle"), "TS_current");
    }

    #[test]
    fn test_compose_copy_text_with_empty_timestamp() {
        assert_eq!(compose_copy_text("", "current", "before"), "_current");
    }

    // --- resolve_timestamp_text ---

    #[test]
    fn test_resolve_paste_returns_timestamp_only() {
        let result = resolve_timestamp_text("paste", "2024-01-01", "", "before").unwrap();
        assert_eq!(result, "2024-01-01");
    }

    #[test]
    fn test_resolve_cut_returns_timestamp_only() {
        let result = resolve_timestamp_text("cut", "2024-01-01", "", "before").unwrap();
        assert_eq!(result, "2024-01-01");
    }

    #[test]
    fn test_resolve_copy_prepends_timestamp() {
        let result = resolve_timestamp_text("copy", "TS", "hello", "before").unwrap();
        assert_eq!(result, "TS_hello");
    }

    #[test]
    fn test_resolve_copy_appends_timestamp() {
        let result = resolve_timestamp_text("copy", "TS", "hello", "after").unwrap();
        assert_eq!(result, "hello_TS");
    }

    #[test]
    fn test_resolve_unknown_action_returns_error() {
        let result = resolve_timestamp_text("delete", "TS", "", "before");
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Unknown timestamp action"));
    }

    #[test]
    fn test_resolve_copy_with_empty_clipboard() {
        let result = resolve_timestamp_text("copy", "TS", "", "before").unwrap();
        assert_eq!(result, "TS_");
    }

    #[test]
    fn test_resolve_copy_with_spaces_in_clipboard() {
        let result =
            resolve_timestamp_text("copy", "TS", "hello world  foo", "before").unwrap();
        assert_eq!(result, "TS_hello world  foo");
    }
}
