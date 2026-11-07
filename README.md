# guif-player-win

[![CI](https://github.com/Az-kijitra/guif-player-win/actions/workflows/ci.yml/badge.svg)](https://github.com/Az-kijitra/guif-player-win/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

`guif-player` は、フォルダー階層のまま音楽を聴く Windows 10 / 11 向けの音楽プレーヤーです(**Tauri 2 + Rust + Svelte 5** 製)。キーボードだけでも、マウスだけでも操作できます。

ファイラー [guif](https://github.com/Az-kijitra/guif) と共通の配色と設計を使っています。

![フォルダービュー](docs/images/folder-view.png)

---

## 主な機能

- **フォルダーのまま聴く**: ライブラリーの登録は不要です。フォルダーを開いて再生します。
- **続きから聴く**: 曲ごとに再生位置を記憶し、次の起動では前回のフォルダーを開きます。
- **タグの編集**: タイトル、アーティスト、アルバム、トラック番号、ジャケットを、複数の曲にまとめて書き込めます。
- **安全な保存**: 一時コピーへ書き込んで音声とタグを確認してから、元のファイルと置き換えます。失敗しても元のファイルは変わりません。
- **アルバムビュー**: フォルダーごとのジャケットを一覧で表示します。
- **プレイリスト**: 作成、追加、削除ができます。
- **続きから聴く**: フォルダーを開くと、最後に聴いていた曲の続きから再生するボタンが出ます。曲ごとの再生位置も覚えます。
- **しおり**: 曲の好きな位置にしおりを挟み、あとでその位置から再生できます。
- **再生速度**: 0.8 倍から 2.0 倍まで、音の高さを変えずに速さを変えられます。
- **Windows のメディア操作**: 曲名とジャケットを音量の表示やロック画面に出し、メディアキーやヘッドセットのボタンで操作できます。
- **シャッフルとリピート**: 全曲を重複なく流すシャッフル、1 曲リピート、全曲リピートがあります。
- **キーボード操作**: 一覧の移動、範囲選択、再生の操作、ビューの切り替えをキーボードだけで行えます。
- **テーマと文字の大きさ**: Nord、Default、Monokai、Catppuccin、Solarized、Light から選べます。guif の設定に合わせることもできます。文字の大きさは 4 段階です。

<table>
  <tr>
    <td><img src="docs/images/artwork-view.png" alt="アルバムビュー"></td>
    <td><img src="docs/images/tag-editor.png" alt="タグの一括編集"></td>
  </tr>
  <tr>
    <td><img src="docs/images/player-detail.png" alt="詳細プレーヤー(再生速度としおり)"></td>
    <td><img src="docs/images/bookmarks.png" alt="しおりの一覧"></td>
  </tr>
</table>

### 対応形式

MP3 (`.mp3`)、FLAC (`.flac`)、AAC / ALAC (`.m4a`、`.aac`)、Ogg Vorbis (`.ogg`)、Opus (`.opus`)、WAV (`.wav`)、AIFF (`.aif`、`.aiff`)

WMA (`.wma`) と、DRM で保護された曲(iTunes Store の古い購入曲や Apple Music の曲など)は再生できません。

---

## インストール

### インストーラーを使う(推奨)

1. [Releases](https://github.com/Az-kijitra/guif-player-win/releases) から最新の `guif-player_<版>_x64-setup.exe` をダウンロードして実行します。
2. 画面の案内に従って進めます。インストールは現在のユーザーだけで、管理者権限は不要です。
3. スタートメニューの「guif-player」から起動します。

> **SmartScreen の警告が出たとき**
> インストーラーにコード署名がないため、「Windows によって PC が保護されました」と表示されることがあります。「詳細情報」を選び、表示される「実行」を押すと進めます。

以前の版から更新するときは、そのまま新しいインストーラーを実行してください。データは引き継がれます。

### ソースからビルドする

Rust 1.90 以上、Node.js 22.17 以上、CMake、Microsoft Edge WebView2 ランタイムが必要です。CMake は Opus のデコーダー(libopus)をビルドするために使います。

> リポジトリのパスが 90 文字ほどを超える深いフォルダーに置くと、ビルドが失敗します。libopus のビルドが Windows のパスの長さの上限(260 文字)を超えるためです。`C:\src` のような浅いフォルダーに置いてください。

```powershell
git clone https://github.com/Az-kijitra/guif-player-win.git
cd guif-player-win/app
npm install
npm run tauri build
```

---

## 主なキー

| キー | 動作 |
|---|---|
| `Space` | 再生 / 一時停止 |
| `↑` `↓` `Enter` | 一覧の移動、再生、フォルダーを開く |
| `Backspace` | 親フォルダーへ |
| `Ctrl+→` / `Ctrl+←` | 次の曲 / 前の曲 |
| `Alt+→` / `Alt+←` | 5 秒進む / 5 秒戻る |
| `Ctrl+1` / `Ctrl+2` / `Ctrl+3` | フォルダー / アルバム / プレイリストのビュー |
| `[` / `]` | 再生速度を遅く / 速く |
| `B` | 今の位置にしおりを挟む |
| `Shift` + 移動キー | 範囲選択 |
| `Ctrl+S` | タグの保存 |

全一覧は[ユーザーマニュアル](docs/user_manual.md#6-キー操作)にあります。

---

## データの保存先

再生位置、プレイリスト、設定は `%LOCALAPPDATA%\dev.guif.player\guif_player.db` に保存します。0.1.1 以前のデータは、起動したときに自動で新しい場所へ移動します。

ネットワークへの送信はしません。

---

## ドキュメント

- [ユーザーマニュアル](docs/user_manual.md): 操作、タグの編集、トラブルシューティング
- [変更履歴](CHANGELOG.md)
- [開発への参加](CONTRIBUTING.md): 開発環境とテスト
- [アーキテクチャ](docs/architecture.md): 内部構造と設計の理由
- [仕様](specs/README.md): 受入基準とテストの対応
- [セキュリティ](SECURITY.md): 脆弱性の報告

---

## ライセンス

[MIT License](LICENSE)

同梱する第三者のソフトウェアは [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) にあります。
