# 仕様

guif-player が守るべき動作と、それを確かめる方法をまとめます。実装の説明は [docs/architecture.md](../docs/architecture.md) にあります。

| 仕様 | 内容 |
|---|---|
| [playback.md](playback.md) | 再生、曲順、対応形式 |
| [tag-editing.md](tag-editing.md) | タグの保存 |
| [data.md](data.md) | データの保存と再生位置 |
| [keyboard.md](keyboard.md) | キー操作 |
| [appearance.md](appearance.md) | テーマ、文字の大きさ |
| [distribution.md](distribution.md) | インストーラー |

## 読み方

受入基準の「確認」の欄は次のどちらかです。

- **`テスト名`**: 自動テストです。`npm run check:docs` がテストの実在を確かめます。
- **手動**: 実機で確認します([CONTRIBUTING.md](../CONTRIBUTING.md))。
- **計画**: まだ実装していません。`npm run check:docs` が一覧を表示します。

ID は [Android 版](https://github.com/Az-kijitra/guif-player)の仕様と同じ番号を、同じ意味で使います。Windows 版で異なる点は、各表の下に書きます。

変更の履歴は Git に任せ、ここには今の仕様だけを書きます。
