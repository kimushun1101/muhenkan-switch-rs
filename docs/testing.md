# テスト

## 自動テスト（cargo test）

### 実行方法

```
cargo test --workspace
```

### テストの場所

- `muhenkan-switch-config/src/lib.rs` — config crate 単体テスト (24件)
- `muhenkan-switch-core/src/commands/timestamp.rs` — timestamp コマンド単体テスト (11件)
- `muhenkan-switch-core/src/commands/open_folder.rs` — フォルダ展開・ゴミ箱パス解決テスト (10件)
- `muhenkan-switch-core/src/commands/switch_app.rs` — Linux ウィンドウ検索テスト (6件, Linux のみ)
- `muhenkan-switch-core/src/commands/toast.rs` — 通知テスト (3件)

### カテゴリ

#### config crate

- **パース** (`test_parse_*`) — TOML デシリアライズ
- **ディスパッチ** (`test_dispatch_*`) — キー→アクション検索、優先順位
- **バリデーション** (`test_validate_*`) — 設定値の検証、キー重複検出
- **Save/Load** (`test_roundtrip_*`, `test_save_*`) — ファイル書き出しと復元、ソート順
- **ヘルパー** (`test_get_*`, `test_app_*`) — ユーティリティ関数

#### CLI crate (muhenkan-switch-core)

- **timestamp** (`test_compose_*`, `test_resolve_*`) — タイムスタンプ結合・アクション解決の純粋ロジック
- **open_folder** — `expand_home` のチルダ展開、`resolve_trash_path` のパス解決、存在しないフォルダのエラー
- **switch_app** — `try_wmctrl`/`try_xdotool` が存在しないアプリでパニックしないこと、`activate_window` のエラーハンドリング
- **toast** — `Toast::show`/`finish` が notify-send 不在でもパニックしないこと、日本語メッセージ対応

### テスト追加時の規約

- テスト名: `test_{カテゴリ}_{何を検証するか}` または `{関数名}_{条件}_{期待結果}`
- 場所: 各 `.rs` ファイル内 `#[cfg(test)] mod tests`
- ファイル I/O を伴うテストは `std::env::temp_dir()` を使用し、末尾で cleanup

---

## 手動テスト（Ubuntu 22.04 X11）

### 前提条件

```bash
# 必須ツール
sudo apt install wmctrl xdotool libnotify-bin xdg-utils

# ビルド
cargo build --workspace

# Trash フォルダが存在するか確認（一度ファイルをゴミ箱に入れると作られる）
ls ~/.local/share/Trash/files/
```

### テスト手順

以下は `muhenkan-switch-core` バイナリを直接 CLI で実行する手順。
kanata 経由のディスパッチテスト（無変換+キー）も同等の動作になる。

#### 1. ゴミ箱を開く（キー 5）

```bash
# CLI 直接実行
cargo run -p muhenkan-switch-core -- open-folder --target trash

# または kanata 経由
# 無変換+5
```

| 条件 | 期待動作 |
|------|---------|
| `~/.local/share/Trash/files/` が存在する | ファイルマネージャ（Nautilus 等）でゴミ箱フォルダが開く |
| `~/.local/share/Trash/files/` が存在しない | エラーメッセージ `Trash folder not found` が表示される |

#### 2. 通常フォルダを開く（キー 1,2,3）

```bash
cargo run -p muhenkan-switch-core -- open-folder --target documents
cargo run -p muhenkan-switch-core -- open-folder --target downloads
cargo run -p muhenkan-switch-core -- open-folder --target desktop
```

| 条件 | 期待動作 |
|------|---------|
| `~/Documents` が存在する | ファイルマネージャでフォルダが開く |
| 存在しないパス | エラーメッセージ `Folder does not exist` が表示される |

#### 3. アプリ切り替え — ブラウザ（キー f）

```bash
# Firefox を事前に起動しておく
cargo run -p muhenkan-switch-core -- switch-app --target browser
```

| 条件 | 期待動作 |
|------|---------|
| Firefox が起動済み | Firefox ウィンドウが最前面にアクティブ化される |
| Firefox が未起動 | Firefox が新規起動される |
| wmctrl 未インストール | xdotool にフォールバックして動作する |
| wmctrl も xdotool も未インストール | launch コマンド実行を試み、失敗しても `Warning:` を stderr に出力して正常終了する |

#### 4. アプリ切り替え — エディタ（キー a）

```bash
# VS Code を事前に起動しておく
cargo run -p muhenkan-switch-core -- switch-app --target editor
```

| 条件 | 期待動作 |
|------|---------|
| VS Code が起動済み | VS Code ウィンドウが最前面にアクティブ化される |
| VS Code が未起動 | `code` コマンドで VS Code が起動される |

#### 5. アプリ切り替え — Office 系（キー w,e,s,d）

```bash
cargo run -p muhenkan-switch-core -- switch-app --target word
```

| 条件 | 期待動作 |
|------|---------|
| デフォルト設定（WINWORD） | `WINWORD` プロセスが見つからず、`winword` を `sh -c` で起動試行 → `Warning:` を stderr に出して正常終了（異常終了しない） |
| config.toml で `process = "libreoffice"` に変更済み | LibreOffice Writer が起動 or アクティブ化される |

#### 6. 通知（toast）

```bash
# notify-send が入っている場合
# タイムスタンプの貼り付けで通知を確認
# 何かテキストをクリップボードにコピーした状態で：
cargo run -p muhenkan-switch-core -- dispatch v
```

| 条件 | 期待動作 |
|------|---------|
| `notify-send` インストール済み | デスクトップ通知が表示される（タイトル: `muhenkan-switch`） |
| `notify-send` 未インストール | 通知なしで正常動作する（サイレントフォールバック） |

#### 7. Web 検索（キー g）

```bash
# テキストをクリップボードにコピーしてから実行
echo -n "Rust programming" | xclip -selection clipboard
cargo run -p muhenkan-switch-core -- search --engine google
```

| 条件 | 期待動作 |
|------|---------|
| クリップボードにテキストあり | ブラウザで Google 検索結果が開く |

#### 8. スクリーンショット（キー p）

```bash
cargo run -p muhenkan-switch-core -- screenshot
```

| 条件 | 期待動作 |
|------|---------|
| flameshot インストール済み | flameshot gui が起動する |
| gnome-screenshot インストール済み | gnome-screenshot が起動する |

### フォールバック検証手順

アプリ切り替えの3段階フォールバックを個別に確認する方法：

```bash
# 1. wmctrl のみで確認
which wmctrl && wmctrl -a firefox
# → Firefox がアクティブ化される

# 2. xdotool --class で確認
xdotool search --class firefox
# → ウィンドウ ID が出力される

# 3. xdotool --name で確認
xdotool search --name firefox
# → ウィンドウ ID が出力される

# 4. wmctrl を一時的に無効にしてフォールバック確認
sudo mv /usr/bin/wmctrl /usr/bin/wmctrl.bak
cargo run -p muhenkan-switch-core -- switch-app --target browser
# → xdotool にフォールバックして Firefox がアクティブ化される
sudo mv /usr/bin/wmctrl.bak /usr/bin/wmctrl
```
