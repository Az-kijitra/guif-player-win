# 仕様

guif-player が守るべき動作と、それを確かめる方法をまとめます。実装の説明は [docs/architecture.md](../docs/architecture.md) にあります。

| 仕様 | 内容 |
|---|---|
| [playback.md](playback.md) | 再生、曲順、対応形式 |
| [tag-editing.md](tag-editing.md) | タグの保存 |
| [data.md](data.md) | データの保存と再生位置 |
| [keyboard.md](keyboard.md) | キー操作 |
| [distribution.md](distribution.md) | インストーラー |

## 読み方

受入基準の「確認」の欄は次のどちらかです。

- **`テスト名`**: 自動テストです。`npm run check:docs` がテストの実在を確かめます。
- **手動**: 実機で確認します([CONTRIBUTING.md](../CONTRIBUTING.md))。

変更の履歴は Git に任せ、ここには今の仕様だけを書きます。
