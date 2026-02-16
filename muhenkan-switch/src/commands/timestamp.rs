use anyhow::Result;
use arboard::Clipboard;
use chrono::Local;
use std::path::{Path, PathBuf};

use super::toast::Toast;
use crate::config::Config;

pub fn run(action: &str, config: &Config) -> Result<()> {
    let timestamp = Local::now().format(&config.timestamp.format).to_string();
    let explorer_hwnd = super::context::get_foreground_explorer_hwnd();

    match (action, explorer_hwnd) {
        // ── V: paste ──
        ("paste", None) => text_paste(&timestamp),
        ("paste", Some(hwnd)) => {
            let toast = Toast::show("処理中...");
            let result = explorer_rename_prepend(&timestamp, &config.timestamp.position, hwnd);
            toast.finish(&format_toast_result(&result));
            result.map(|_| ())
        }

        // ── C: copy (Explorer only) ──
        ("copy", Some(hwnd)) => {
            let toast = Toast::show("処理中...");
            let result = explorer_duplicate(&timestamp, &config.timestamp.position, hwnd);
            toast.finish(&format_toast_result(&result));
            result.map(|_| ())
        }
        ("copy", None) => Ok(()),

        // ── X: cut (Explorer only) ──
        ("cut", Some(hwnd)) => {
            let toast = Toast::show("処理中...");
            let result = explorer_rename_remove(&timestamp, &config.timestamp.position, hwnd);
            toast.finish(&format_toast_result(&result));
            result.map(|_| ())
        }
        ("cut", None) => Ok(()),

        _ => anyhow::bail!(
            "Unknown timestamp action: '{}'. Use paste, copy, or cut.",
            action
        ),
    }
}

fn format_toast_result(result: &Result<Vec<PathBuf>>) -> String {
    match result {
        Ok(paths) if paths.is_empty() => "(no selection)".to_string(),
        Ok(paths) if paths.len() == 1 => {
            let name = paths[0]
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            format!("\u{2713} {}", name)
        }
        Ok(paths) => format!("\u{2713} {} files", paths.len()),
        Err(e) => format!("\u{2717} {}", e),
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

// ── Explorer コンテキスト (COM API 直接呼び出し) ──

/// COM API を通じて Explorer ウィンドウの選択ファイルパスを取得
#[cfg(target_os = "windows")]
fn get_selected_paths(hwnd: isize) -> Result<Vec<PathBuf>> {
    use windows::core::Interface;
    use windows::Win32::Foundation::HWND;
    use windows::Win32::System::Com::{
        CoCreateInstance, CoInitializeEx, CoTaskMemFree, IServiceProvider, CLSCTX_LOCAL_SERVER,
        COINIT_APARTMENTTHREADED,
    };
    use windows::Win32::System::Ole::IOleWindow;
    use windows::Win32::System::Variant::VARIANT;
    use windows::Win32::UI::Shell::{
        IFolderView2, IShellItem, IShellItemArray, IShellWindows, ShellWindows,
        SID_STopLevelBrowser, SIGDN_FILESYSPATH,
    };
    use windows::Win32::UI::WindowsAndMessaging::{GetAncestor, GA_ROOT};

    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);

        let shell_windows: IShellWindows =
            CoCreateInstance(&ShellWindows, None, CLSCTX_LOCAL_SERVER)?;

        let count = shell_windows.Count()?;
        let target = HWND(hwnd as *mut _);

        for i in 0..count {
            let v = VARIANT::from(i);
            let disp = match shell_windows.Item(&v) {
                Ok(d) => d,
                Err(_) => continue,
            };

            let sp: IServiceProvider = match disp.cast() {
                Ok(s) => s,
                Err(_) => continue,
            };

            let browser: windows::Win32::UI::Shell::IShellBrowser =
                match sp.QueryService(&SID_STopLevelBrowser) {
                    Ok(b) => b,
                    Err(_) => continue,
                };

            let ole: IOleWindow = browser.cast()?;
            let wnd = ole.GetWindow()?;
            let root = GetAncestor(wnd, GA_ROOT);
            if wnd != target && root != target {
                continue;
            }

            let view = browser.QueryActiveShellView()?;
            let fv: IFolderView2 = view.cast()?;
            let items: IShellItemArray = match fv.GetSelection(false) {
                Ok(items) => items,
                Err(_) => return Ok(vec![]),
            };

            let item_count = items.GetCount()?;
            let mut paths = Vec::with_capacity(item_count as usize);

            for j in 0..item_count {
                let item: IShellItem = items.GetItemAt(j)?;
                let name_pwstr = item.GetDisplayName(SIGDN_FILESYSPATH)?;
                let path_string = name_pwstr.to_string()?;
                CoTaskMemFree(Some(name_pwstr.0 as _));
                paths.push(PathBuf::from(path_string));
            }

            return Ok(paths);
        }

        Ok(vec![])
    }
}

#[cfg(not(target_os = "windows"))]
fn get_selected_paths(_hwnd: isize) -> Result<Vec<PathBuf>> {
    anyhow::bail!("Explorer file operations are only supported on Windows")
}

/// V: ファイル名にタイムスタンプを付加してリネーム
fn explorer_rename_prepend(timestamp: &str, position: &str, hwnd: isize) -> Result<Vec<PathBuf>> {
    let paths = get_selected_paths(hwnd)?;
    let mut results = Vec::with_capacity(paths.len());
    for src in &paths {
        let dst = build_timestamped_path(src, timestamp, position);
        std::fs::rename(src, &dst)?;
        results.push(dst);
    }
    Ok(results)
}

/// C: タイムスタンプ付きファイル名で複製
fn explorer_duplicate(timestamp: &str, position: &str, hwnd: isize) -> Result<Vec<PathBuf>> {
    let paths = get_selected_paths(hwnd)?;
    let mut results = Vec::with_capacity(paths.len());
    for src in &paths {
        let dst = build_timestamped_path(src, timestamp, position);
        std::fs::copy(src, &dst)?;
        results.push(dst);
    }
    Ok(results)
}

/// X: ファイル名から本日のタイムスタンプを除去してリネーム
fn explorer_rename_remove(timestamp: &str, position: &str, hwnd: isize) -> Result<Vec<PathBuf>> {
    let paths = get_selected_paths(hwnd)?;
    let mut results = Vec::new();
    for src in &paths {
        if let Some(dst) = build_removed_timestamp_path(src, timestamp, position) {
            std::fs::rename(src, &dst)?;
            results.push(dst);
        }
    }
    Ok(results)
}

/// タイムスタンプを付加したファイルパスを構築
fn build_timestamped_path(src: &Path, timestamp: &str, position: &str) -> PathBuf {
    let stem = src.file_stem().unwrap_or_default().to_string_lossy();
    let ext = src
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let new_name = if position == "after" {
        format!("{}_{}{}", stem, timestamp, ext)
    } else {
        format!("{}_{}{}", timestamp, stem, ext)
    };

    src.with_file_name(new_name)
}

/// タイムスタンプを除去したファイルパスを構築 (一致しなければ None)
fn build_removed_timestamp_path(src: &Path, timestamp: &str, position: &str) -> Option<PathBuf> {
    let stem = src.file_stem()?.to_string_lossy();
    let ext = src
        .extension()
        .map(|e| format!(".{}", e.to_string_lossy()))
        .unwrap_or_default();

    let new_stem = if position == "after" {
        let suffix = format!("_{}", timestamp);
        stem.strip_suffix(&*suffix)?.to_string()
    } else {
        let prefix = format!("{}_", timestamp);
        stem.strip_prefix(&*prefix)?.to_string()
    };

    Some(src.with_file_name(format!("{}{}", new_stem, ext)))
}
