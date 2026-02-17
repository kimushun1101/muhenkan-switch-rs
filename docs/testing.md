# テスト

## 実行方法

```
cargo test --workspace
```

## テストの場所

- `muhenkan-switch-config/src/lib.rs` — config crate 単体テスト (24件)
- `muhenkan-switch-core/src/commands/timestamp.rs` — timestamp コマンド単体テスト (11件)

## カテゴリ

### config crate

- **パース** (`test_parse_*`) — TOML デシリアライズ
- **ディスパッチ** (`test_dispatch_*`) — キー→アクション検索、優先順位
- **バリデーション** (`test_validate_*`) — 設定値の検証、キー重複検出
- **Save/Load** (`test_roundtrip_*`, `test_save_*`) — ファイル書き出しと復元、ソート順
- **ヘルパー** (`test_get_*`, `test_app_*`) — ユーティリティ関数

### CLI crate (muhenkan-switch-core)

- **timestamp** (`test_compose_*`, `test_resolve_*`) — タイムスタンプ結合・アクション解決の純粋ロジック

## テスト追加時の規約

- テスト名: `test_{カテゴリ}_{何を検証するか}`
- 場所: 各 crate の `src/lib.rs` 内 `#[cfg(test)] mod tests`
- ファイル I/O を伴うテストは `std::env::temp_dir()` を使用し、末尾で cleanup
