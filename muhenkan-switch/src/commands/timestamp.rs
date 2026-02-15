use anyhow::Result;
use arboard::Clipboard;
use chrono::Local;
use std::time::Duration;

use crate::config::Config;

/// timestamp と current を position に応じて結合
fn compose_text(timestamp: &str, current: &str, position: &str) -> String {
    match position {
        "after" => format!("{}_{}", current, timestamp),
        _ => format!("{}_{}", timestamp, current),
    }
}

/// テキストから本日のタイムスタンプを除去
fn remove_timestamp(text: &str, timestamp: &str, position: &str) -> String {
    match position {
        "after" => text
            .strip_suffix(&format!("_{}", timestamp))
            .unwrap_or(text)
            .to_string(),
        _ => text
            .strip_prefix(&format!("{}_", timestamp))
            .unwrap_or(text)
            .to_string(),
    }
}

pub fn run(action: &str, config: &Config) -> Result<()> {
    let timestamp = Local::now().format(&config.timestamp.format).to_string();
    let explorer_hwnd = super::context::get_foreground_explorer_hwnd();

    match (action, explorer_hwnd) {
        // ── V: paste ──
        ("paste", None) => text_paste(&timestamp),
        ("paste", Some(hwnd)) => {
            explorer_rename_prepend(&timestamp, &config.timestamp.position, hwnd)
        }

        // ── C: copy ──
        ("copy", None) => text_copy(&timestamp, &config.timestamp.position),
        ("copy", Some(hwnd)) => {
            explorer_duplicate(&timestamp, &config.timestamp.position, hwnd)
        }

        // ── X: cut ──
        ("cut", None) => text_remove(&timestamp, &config.timestamp.position),
        ("cut", Some(hwnd)) => {
            explorer_rename_remove(&timestamp, &config.timestamp.position, hwnd)
        }

        _ => anyhow::bail!(
            "Unknown timestamp action: '{}'. Use paste, copy, or cut.",
            action
        ),
    }
}

// ── テキスト入力コンテキスト ──

/// V: タイムスタンプをカーソル位置に貼り付け
fn text_paste(timestamp: &str) -> Result<()> {
    let mut clipboard = Clipboard::new()?;
    clipboard.set_text(timestamp)?;
    super::keys::simulate_paste()?;
    Ok(())
}

/// C: 選択テキストをコピー → タイムスタンプと結合 → クリップボードへ
fn text_copy(timestamp: &str, position: &str) -> Result<()> {
    super::keys::simulate_copy()?;
    std::thread::sleep(Duration::from_millis(200));
    let mut clipboard = Clipboard::new()?;
    let current = clipboard.get_text().unwrap_or_default();
    let text = compose_text(timestamp, &current, position);
    clipboard.set_text(&text)?;
    Ok(())
}

/// X: 選択テキストから本日の日付を除去して貼り戻す
fn text_remove(timestamp: &str, position: &str) -> Result<()> {
    super::keys::simulate_copy()?;
    std::thread::sleep(Duration::from_millis(200));
    let mut clipboard = Clipboard::new()?;
    let current = clipboard.get_text().unwrap_or_default();
    let cleaned = remove_timestamp(&current, timestamp, position);
    clipboard.set_text(&cleaned)?;
    super::keys::simulate_paste()?;
    Ok(())
}

// ── Explorer コンテキスト (Shell.Application COM 経由) ──

/// Explorer の選択ファイルに対して操作を実行する共通ヘルパー
/// HWND は Rust 側で取得済みなので Add-Type 不要
#[cfg(target_os = "windows")]
fn run_explorer_script(per_item_body: &str, hwnd: isize) -> Result<()> {
    use std::process::Command;

    let script = r#"
$fgHwnd = __HWND__
$shell = New-Object -ComObject Shell.Application
foreach ($w in $shell.Windows()) {
    if ($w.HWND -eq $fgHwnd) {
        foreach ($item in $w.Document.SelectedItems()) {
            $src = $item.Path
            $dir = Split-Path $src
            $name = [IO.Path]::GetFileNameWithoutExtension($src)
            $ext = [IO.Path]::GetExtension($src)
            __BODY__
        }
        break
    }
}
"#
    .replace("__HWND__", &hwnd.to_string())
    .replace("__BODY__", per_item_body);

    Command::new("powershell")
        .args(["-NoProfile", "-Command", &script])
        .output()?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn run_explorer_script(_per_item_body: &str, _hwnd: isize) -> Result<()> {
    anyhow::bail!("Explorer file operations are only supported on Windows")
}

/// V: ファイル名にタイムスタンプを付加してリネーム
fn explorer_rename_prepend(timestamp: &str, position: &str, hwnd: isize) -> Result<()> {
    let body = if position == "after" {
        format!(
            "$newName = $name + '_{ts}' + $ext\nRename-Item -LiteralPath $src -NewName $newName",
            ts = timestamp
        )
    } else {
        format!(
            "$newName = '{ts}_' + $name + $ext\nRename-Item -LiteralPath $src -NewName $newName",
            ts = timestamp
        )
    };
    run_explorer_script(&body, hwnd)
}

/// C: タイムスタンプ付きファイル名で複製
fn explorer_duplicate(timestamp: &str, position: &str, hwnd: isize) -> Result<()> {
    let body = if position == "after" {
        format!(
            "$newName = $name + '_{ts}' + $ext\n$dst = Join-Path $dir $newName\nCopy-Item -LiteralPath $src -Destination $dst",
            ts = timestamp
        )
    } else {
        format!(
            "$newName = '{ts}_' + $name + $ext\n$dst = Join-Path $dir $newName\nCopy-Item -LiteralPath $src -Destination $dst",
            ts = timestamp
        )
    };
    run_explorer_script(&body, hwnd)
}

/// X: ファイル名から本日のタイムスタンプを除去してリネーム
fn explorer_rename_remove(timestamp: &str, position: &str, hwnd: isize) -> Result<()> {
    let body = if position == "after" {
        format!(
            r#"if ($name.EndsWith('_{ts}')) {{
    $newName = $name.Substring(0, $name.Length - {len}) + $ext
    Rename-Item -LiteralPath $src -NewName $newName
}}"#,
            ts = timestamp,
            len = timestamp.len() + 1
        )
    } else {
        format!(
            r#"if ($name.StartsWith('{ts}_')) {{
    $newName = $name.Substring({len}) + $ext
    Rename-Item -LiteralPath $src -NewName $newName
}}"#,
            ts = timestamp,
            len = timestamp.len() + 1
        )
    };
    run_explorer_script(&body, hwnd)
}

#[cfg(test)]
mod tests {
    use super::*;

    // --- compose_text ---

    #[test]
    fn test_compose_text_before() {
        assert_eq!(compose_text("TS", "current", "before"), "TS_current");
    }

    #[test]
    fn test_compose_text_after() {
        assert_eq!(compose_text("TS", "current", "after"), "current_TS");
    }

    #[test]
    fn test_compose_text_unknown_position_defaults_to_before() {
        assert_eq!(compose_text("TS", "current", "middle"), "TS_current");
    }

    #[test]
    fn test_compose_text_with_empty_timestamp() {
        assert_eq!(compose_text("", "current", "before"), "_current");
    }

    // --- remove_timestamp ---

    #[test]
    fn test_remove_timestamp_before() {
        assert_eq!(remove_timestamp("TS_hello", "TS", "before"), "hello");
    }

    #[test]
    fn test_remove_timestamp_after() {
        assert_eq!(remove_timestamp("hello_TS", "TS", "after"), "hello");
    }

    #[test]
    fn test_remove_timestamp_not_found() {
        assert_eq!(remove_timestamp("hello", "TS", "before"), "hello");
    }

    #[test]
    fn test_remove_timestamp_different_date_not_removed() {
        assert_eq!(
            remove_timestamp("20250101_hello", "20260216", "before"),
            "20250101_hello"
        );
    }

    #[test]
    fn test_remove_timestamp_after_not_found() {
        assert_eq!(remove_timestamp("hello", "TS", "after"), "hello");
    }
}
