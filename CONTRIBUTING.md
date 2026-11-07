# 開発への参加

不具合の報告や修正を歓迎します。内部構造は [docs/architecture.md](docs/architecture.md)、機能の要件は [specs/](specs/README.md) にあります。

セキュリティの問題は Issue に書かず、[SECURITY.md](SECURITY.md) の方法で連絡してください。

## 環境

- Windows 10 / 11(64 ビット版)
- Rust 1.90 以上
- Node.js 22.17 以上
- Microsoft Edge WebView2 ランタイム

```powershell
cd app
npm install
npm run tauri dev
```

## 変更する前に確認すること

CI と同じ内容です。すべて `app` フォルダーで実行します。

```powershell
npm test                  # フロントエンドのテスト
npm run check             # 型チェック
npx tsc --noEmit          # 型チェック
npm run check:docs        # ドキュメントのリンクと仕様のテスト名

cd src-tauri
cargo fmt --check
cargo clippy --all-targets --locked -- -D warnings
cargo test --locked
```

CI はこのほかに、`npm audit --omit=dev` と `cargo-deny` で依存関係を検査します。

依存関係を変えたときは `npm run notices` で [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md) を作り直してください。

## 実データを使わずにアプリを動かす

デバッグビルドでは、環境変数 `GUIF_PLAYER_DATA_DIR` でデータの保存先を変えられます。リリースビルドでは無視します。

```powershell
$env:GUIF_PLAYER_DATA_DIR = "$env:TEMP\guif-player-test"
npm run tauri dev
```

## 守ってほしいこと

- **音楽ファイルを壊さない。** タグの書き込みを変えるときは、[docs/architecture.md](docs/architecture.md) の手順を保ち、実ファイルのコピーでも確認してください。
- **メインスレッドで待たない。** ファイルアクセスやプレーヤーのロックを取るコマンドは `run_blocking` か `with_player` を通します。
- **出荷した DB の手順を変えない。** スキーマを変えるときは `MIGRATIONS` に追加します。
- **音楽ファイルをリポジトリに入れない。** テスト用の音声は `test_audio.rs` で作ります。
- **不具合の修正には再現テストを付ける。**
- **文言は日本語。** 画面の文言とドキュメントは日本語、コードのコメントとテスト名は英語です。

## 手動で確認すること

自動テストで確かめられないものです。次のようなものを変えたときは、実機で確認してください。

| 変更 | 確認 |
|---|---|
| 再生まわり | 再生、シーク、曲の終わり、シャッフル、リピート |
| タグの保存 | 再生中の曲の保存、読み取り専用のファイルを含む一括保存 |
| CSP | `npx tauri build --debug --no-bundle` で作った実行ファイルで、一覧、アートワーク、再生 |
| キー操作 | 3 つのビューでの移動、選択、ダイアログからの復帰 |

## リリース

手順は [docs/release.md](docs/release.md) にあります。

## コミットとプルリクエスト

コミットの題は `fix:`、`feat:`、`perf:`、`docs:`、`ci:`、`chore:` などで始め、日本語で何をしたかを書きます。

プルリクエストには、変更の理由と確認した内容を書いてください。利用者に関わる変更は [CHANGELOG.md](CHANGELOG.md) にも追加します。
