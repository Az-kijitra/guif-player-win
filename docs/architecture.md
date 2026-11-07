# アーキテクチャ

guif-player の内部構造と、設計上の判断の理由をまとめます。機能の要件は [specs/](../specs/README.md) にあります。

## 全体像

Tauri 2 のデスクトップアプリです。画面は Svelte 5、音声とファイル処理は Rust が担当します。

```
┌─ WebView2 ───────────────────────────┐
│ routes/+page.svelte  キー操作の入口   │
│ lib/components/*     各画面           │
│ lib/playerState.svelte.ts  状態と操作 │
│ lib/api.ts           コマンドの呼び出し│
└──────────────┬───────────────────────┘
               │ invoke / イベント
┌─ Rust ───────┴───────────────────────┐
│ lib.rs       コマンド、背景スレッド   │
│ player.rs    再生、デコード           │
│ queue.rs     曲順、シャッフル、リピート│
│ metadata.rs  タグの読み書き           │
│ scanner.rs   フォルダーの走査         │
│ db.rs        SQLite                   │
└──────────────────────────────────────┘
```

| ライブラリ | 用途 |
|---|---|
| [rodio](https://crates.io/crates/rodio) | 音声出力 |
| [symphonia](https://crates.io/crates/symphonia) | デコード |
| [symphonia-adapter-libopus](https://crates.io/crates/symphonia-adapter-libopus) | Opus のデコード(libopus)。symphonia 0.5 には Opus のデコーダーがないため |
| [wsola](https://crates.io/crates/wsola) | 音の高さを保った再生速度の変更 |
| [lofty](https://crates.io/crates/lofty) | タグの読み書き |
| [rusqlite](https://crates.io/crates/rusqlite) | SQLite |

## スレッドとロック

時間のかかる処理を、メインスレッドで動かしません。Tauri の `async` でないコマンドはメインスレッドで動き、そこで待つとウィンドウが固まるためです。

| スレッド | 処理 |
|---|---|
| メイン | ウィンドウ。DB の短い読み書き |
| ブロッキングプール(`run_blocking`) | フォルダーの走査、タグ、アートワーク、プレーヤーのロックを取る操作 |
| 背景スレッド(`run` 内、400ms ごと) | 曲の終わりの検出と次の曲への移動、再生位置の保存(3 秒ごと) |
| rodio の音声スレッド | 音声の出力 |

プレーヤーの状態は `Arc<Mutex<AudioPlayer>>` です。曲を開くデコードはこのロックの中で動くので、ロックを待つコマンドはすべて `with_player` を通してメインスレッドの外で実行します。

### 画面との同期

画面は 250ms ごとに `get_playback_state` で状態を取得します。何度も送るので、この応答に再生キュー全体は含めません。`queue_version` だけを送り、変わったときに `get_queue` で取得します。

走査はメインスレッドの外で動くので、古い要求の結果があとから届くことがあります。`navigateTo` と `loadAlbums` は要求に番号を付け、最新でない結果を捨てます。

自動で次の曲へ進むときの失敗は、応答を返す相手がいないので、イベント `playback-error` で画面へ送ります。

## 再生

`SymphoniaSource` は symphonia のデコーダーを rodio の `Source` にしたものです。曲順の規則は音声出力と切り離して `PlayQueue`(queue.rs)にあり、音声デバイスがなくてもテストできます。

再生位置は実時間から計算しています(`start_time` と `elapsed_before_pause`)。

### ファイルを開くときの代替経路

通常の読み込みに失敗したときは、順に次を試します。

1. **`udta` を飛ばして開く(`Mp4WithoutUserData`)**: タグ編集ソフトが `udta` の中に読めないデータを残した M4A は、symphonia がファイルごと開けません。再生に `udta` は不要で、タグは lofty で読みます。そこで読み込むときだけ `udta` を `free` に見せます。ファイルは書き換えません。
2. **MP3 として開く**: RIFF で包まれた MP3 のためです。

どれも失敗したときは、MP4 の `stsd` を見て、`drms` や `enca` なら DRM の曲として知らせます。DRM の解除はしません。

## タグの書き込み

音楽ファイルを壊さないことを最優先にしています。手順は `update_file_tags`(metadata.rs)にあります。

1. 読み取り専用なら拒否します。
2. 同じフォルダーに一時コピー `<名前>.guif-player-tmp.<拡張子>` を作ります。一覧には表示しません。
3. コピーへ書き込みます。
4. `check_written_file` で、再生時間、先頭 5 秒のデコード結果、タグの読み戻しを確かめます。
5. `ReplaceFileW` で元のファイルと置き換えます。作成日時、属性、権限が残ります。

再生中の曲は、書き込みの前に閉じ(`suspend_if_playing`)、同じ位置から再開します(`restore_after_tag_update`)。

### MP3 と M4A は専用の API で書く

lofty の汎用の `Tag` を通すと、次のことが起きます。そのため MP3 は `Id3v2Tag`、M4A は `Ilst` を直接操作します。

| 問題 | 例 |
|---|---|
| 汎用の形式にない情報が消える | 形式の崩れたトラック番号(`11/`) |
| ID3v2.3 が v2.4 に変わる | v2.4 を読めない古いソフトで問題になる |

MP3 はさらに次のようにしています。

- **ID3v2 タグだけを保存します。** ファイル全体を保存すると lofty が APE タグも書き直し、ヘッダーのない APE タグでは音声の末尾 32 バイトを削ってしまいます。
- **ID3v2.3 は v2.3 のまま保存します。** ただし v2.3 に残せないフレーム(`ID3V24_ONLY_FRAMES`)があれば、情報を失わないように v2.4 にします。iTunes は v2.3 のタグにも `TDRL` などを書くためです。
- **不正な言語コードを `XXX` に直します。** lofty は不正な言語コードがあると保存全体を拒否します。

### 依存ライブラリの注意

lofty 0.21 は、末尾に PADDING のない FLAC の音声を壊します。0.25 で修正されています。回帰テストは `flac_audio_survives_every_metadata_layout` です。

symphonia は、後ろにチャンクがある AIFF で SSND の終端を 8 バイト読み過ぎます。保存の確認で音声を前方一致で比べているのはこのためです。

## データベース

`%LOCALAPPDATA%\dev.guif.player\guif_player.db` です。フォルダー名を Tauri の `identifier` と同じにしているのは、アンインストーラーのデータ削除の対象にするためです。

| テーブル | 内容 |
|---|---|
| `album_resume` | フォルダーごとの最後の曲と位置 |
| `track_resume` | 曲ごとの位置と、聴き終えたか |
| `playlists`、`playlist_tracks` | プレイリストと曲。曲の情報は複製して持ちます |
| `app_settings` | テーマなどの設定 |

### スキーマの変更

`MIGRATIONS`(db.rs)に手順を追加します。**出荷した手順は変更しません。** 版は `PRAGMA user_version` に記録し、手順ごとに版と一緒にコミットします。アプリより新しい版のデータベースは開きません。

## セキュリティ

- 画面は外部のコンテンツを読み込みません。CSP(`tauri.conf.json`)で、外部への接続と外部のスクリプトを禁止しています。
- アートワークは `data:` URL で画面に渡すので、`img-src` で `data:` を許可しています。
- Svelte がエスケープするので、タグの文字列は HTML として解釈されません。`{@html}` は使いません。

アプリのコマンドは、Tauri の権限でコマンドごとに制限していません。

## テスト

| 種類 | 場所 | 内容 |
|---|---|---|
| Rust の単体テスト | 各 `.rs` の `tests` | 曲順、DB、タグ、ファイルの判定 |
| Vitest | `app/src/**/*.test.ts` | キー移動、状態管理、エラー表示、テーマのコントラスト |

テスト用の音声ファイルは `test_audio.rs` がメモリ上で作ります。音楽ファイルをリポジトリに置かないためです。

インストーラーは Windows Sandbox の中で実際に確かめます([docs/release.md](release.md))。

音声デバイスと実ファイルが必要な確認は手動です。手順は [CONTRIBUTING.md](../CONTRIBUTING.md) にあります。
