# guif-player-win

Windows 版 `guif-player` — キーボード＆マウス対応のモダンな音楽プレイヤー（**Tauri 2 + Rust + Svelte 5**）。

CUI/TUI ファイラー `tuif`、および Windows ファイラー `guif` の Nord テーマ・UI 設計を踏襲し、フォルダ階層によるブラウズ、アルバムアート一覧、プレイリスト管理、レジューム再生を備えています。

---

## 主な機能

- **フォルダブラウズ**: ディレクトリ構造を維持した音楽ファイルの探索
- **アートワークグリッド**: アルバムごとのジャケット写真一覧表示
- **プレイリスト**: 複数トラックのプレイリスト作成・保存・管理
- **レジューム再生**: アルバムおよびトラック単位の再生位置記憶と自動復帰
- **Nord テーマ**: Polar Night / Frost / Aurora を基調とした視認性の高いUI
- **キーボードナビゲーション**: 再生/一時停止（Space）、シーク、トラック移動、ビュー切替

---

## 開発環境のセットアップ

### 必要要件
- Windows 10 / 11 (64bit)
- Rust 1.75 以上
- Node.js 20 以上
- Microsoft Edge WebView2 ランタイム

### 起動方法
```powershell
cd app
npm install
npm run tauri dev
```
