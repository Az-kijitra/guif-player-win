# リリース手順

Windows の開発環境で行います。

## 1. 版を上げる

次のファイルの版を同じ値にします。`npm run release:check` が食い違いを検出します。CI でも確かめます。

- `app/src-tauri/tauri.conf.json`
- `app/package.json`、`app/package-lock.json`
- `app/src-tauri/Cargo.toml`、`app/src-tauri/Cargo.lock`
- `CHANGELOG.md`(いちばん上の見出し)

依存関係を変えたときは `npm run notices` も実行します。

## 2. インストーラーを作る

```powershell
cd app
npm run release:build
```

`app/src-tauri/target/release/bundle/nsis/` に次のファイルができます。古いインストーラーは削除されるので、更新の確認に使う旧版は別の場所に保管してください。

- `guif-player_<版>_x64-setup.exe`
- `guif-player_<版>_x64-setup.exe.sha256`

## 3. インストーラーを確かめる

Windows Sandbox の中で、実際にインストールして確かめます。この PC とデータには触れません。Windows の機能「Windows サンドボックス」が必要です。実行中はサンドボックスのウィンドウが表示され、終わると閉じます。

```powershell
npm run release:sandbox -- <旧版のインストーラー>
```

旧版からの更新を確かめないときは、引数を省略します。結果は最後に表示され、画面の記録が結果のフォルダーに残ります。

確かめる項目は [specs/distribution.md](../specs/distribution.md) にあります。

サンドボックスには音声デバイスがないことがあるので、再生は開発環境で確かめます([CONTRIBUTING.md](../CONTRIBUTING.md))。

## 4. 公開する

1. `main` へマージし、CI が通ることを確かめます。
2. タグ `v<版>` を付けます。
3. GitHub の Release を作り、`CHANGELOG.md` のその版の内容を本文にして、インストーラーと `.sha256` を添付します。
