# guif-player-win

[![CI](https://github.com/Az-kijitra/guif-player-win/actions/workflows/ci.yml/badge.svg)](https://github.com/Az-kijitra/guif-player-win/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

Windows 版 `guif-player` — キーボード＆マウス対応のモダンな音楽プレイヤー（**Tauri 2 + Rust + Svelte 5**）。

CUI/TUI ファイラー `tuif`、および Windows ファイラー `guif` の Nord テーマ・UI 設計を踏襲し、フォルダ階層によるブラウズ、アルバムアート一覧、プレイリスト管理、レジューム再生を備えています。

---

## 主な機能

- **フォルダブラウズ**: ディレクトリ構造を維持した音楽ファイルの探索
- **アートワークグリッド**: アルバムごとのジャケット写真一覧表示
- **プレイリスト**: 複数トラックのプレイリスト作成・保存・管理
- **レジューム再生**: アルバムおよびトラック単位の再生位置記憶と自動復帰
- **シャッフルとリピート**: 全曲を一度ずつ流すシャッフル、1曲リピート、全曲リピート
- **Nord テーマ**: Polar Night / Frost / Aurora を基調とした視認性の高いUI
- **キーボード操作**: 一覧の移動と選択、再生の操作、ビュー切替をキーボードだけで行えます

### 対応形式

MP3 (`.mp3`)、FLAC (`.flac`)、AAC / ALAC (`.m4a`、`.aac`)、Ogg Vorbis (`.ogg`)、WAV (`.wav`)、AIFF (`.aif`、`.aiff`)

Opus (`.opus`) には今後対応する予定です。WMA (`.wma`) と、DRM で保護された曲（iTunes Store の古い購入曲や Apple Music の曲など）は再生できません。

---

## キー操作

### どこでも使えるキー

| キー | 動作 |
|---|---|
| `Space` | 再生 / 一時停止 |
| `Ctrl+→` / `Ctrl+←` | 次の曲 / 前の曲 |
| `Alt+→` / `Alt+←` | 5秒進む / 5秒戻る |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | フォルダ / アートワーク / プレイリストのビューへ切替 |
| `Backspace` / `Alt+↑` | 親フォルダへ（フォルダビュー） |
| `Ctrl+A` | すべての曲を選択 |
| `Ctrl+I` | プロパティパネルの表示 / 非表示 |
| `Ctrl+S` | プロパティの保存 |
| `Esc` | ダイアログを閉じる / 一覧へ戻る |

### 一覧のキー

| キー | 動作 |
|---|---|
| `↑` / `↓` | 上下へ移動（アートワークでは `←` / `→` も使えます） |
| `PageUp` / `PageDown` | 1画面分移動 |
| `Home` / `End` | 先頭 / 末尾へ移動 |
| `Enter` | 曲を再生 / フォルダやアルバムを開く |
| `Shift` + 移動キー | 選択範囲を広げる |
| `Ctrl+Space` | 選択の切替 |
| `I` | プロパティを編集 |
| `P` | プレイリストに追加 |

### プレイリストビューのキー

| キー | 動作 |
|---|---|
| `Enter`（左の一覧） | プレイリストを開いて曲の一覧へ移動 |
| `→` / `←` | 左の一覧と曲の一覧の間を移動 |
| `Delete` | プレイリストを削除（確認あり） / 曲をプレイリストから外す |

---

## データの保存先

再生位置、プレイリスト、設定は `%LOCALAPPDATA%\dev.guif.player\guif_player.db` に保存します。

0.1.1 以前は `%LOCALAPPDATA%\guif-player\` に保存していました。起動すると、データは自動で新しい場所へ移動します。

---

## 開発環境のセットアップ

### 必要要件
- Windows 10 / 11 (64bit)
- Rust 1.90 以上
- Node.js 22.12 以上
- Microsoft Edge WebView2 ランタイム

### 起動方法
```powershell
cd app
npm install
npm run tauri dev
```

### テスト
```powershell
cd app
npm test                                         # フロントエンド (Vitest)
npm run check                                    # フロントエンドの型チェック
cargo test --manifest-path src-tauri/Cargo.toml  # バックエンド (Rust)
```

プッシュすると CI が、これらに加えて `cargo fmt --check`、`cargo clippy -- -D warnings`、依存関係の検査（`npm audit`、`cargo-deny`）を実行します。

---

## ライセンス

[MIT License](LICENSE)

同梱する第三者のソフトウェアは [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) にあります。
