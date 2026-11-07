# 再生

## 曲順

| ID | 受入基準 | 確認 |
|---|---|---|
| P-1 | リピートがオフなら、最後の曲の次へは進まない | `next_stops_at_the_end_when_repeat_is_off` |
| P-2 | 全曲リピートなら、最後の曲の次は先頭に戻る | `next_wraps_around_when_repeat_all_is_on` |
| P-3 | シャッフルは全曲を重複なく一度ずつ再生してから止まる | `shuffle_plays_every_track_once_and_then_stops` |
| P-4 | シャッフルと全曲リピートでは、同じ曲が続かない | `shuffle_with_repeat_all_keeps_going_without_an_immediate_repeat` |
| P-5 | シャッフル中の「前の曲」は再生した順に戻る | `shuffle_previous_walks_back_and_next_returns_to_the_same_track` |
| P-6 | リピートはオフ、全曲、1 曲の順に切り替わる | `repeat_mode_cycles_off_all_one_off` |
| P-7 | シャッフルなしの全曲再生は、前回のシャッフルを引き継がない | `plays everything in order with shuffle off`、`play_all_without_shuffle_does_not_keep_the_earlier_shuffle` |
| P-8 | シャッフル再生は、シャッフルをオンにして無作為な曲から始める。見つからない曲からは始めない | `plays everything shuffled from a random track`、`does not start on a missing track` |

P-8 は Windows 版だけの基準です。Android 版のシャッフル再生は先頭の曲から始めます。

## 曲の終わり

| ID | 受入基準 | 確認 |
|---|---|---|
| P-10 | キューの最後の曲が終わると再生は止まり、同じ処理を繰り返さない | `last_track_ending_stops_playback_once` |
| P-11 | 1 曲リピートでは同じ曲をもう一度再生する | 手動 |
| P-12 | 再生できない曲は飛ばして、残りの曲を試す | `a_track_that_cannot_be_played_leaves_the_rest_of_the_queue_to_try` |
| P-13 | 全曲リピートで全曲が失敗したら止まる | `repeat_all_stops_once_every_track_has_failed` |
| P-14 | 再生が止まったあとの再生操作は、最後の曲をもう一度再生する | `toggling_after_the_end_plays_the_last_track_again` |

## 対応形式

| ID | 受入基準 | 確認 |
|---|---|---|
| P-20 | 再生できる形式だけを一覧に表示する | `playable_formats_are_listed`、`formats_without_a_decoder_are_not_listed` |
| P-21 | WAV と AIFF をデコードできる | `decodes_wav`、`decodes_aiff` |
| P-22 | タグの領域に読めないデータがある M4A を、ファイルを変えずに再生できる | `the_atom_reads_as_free_however_the_reads_are_split_and_the_file_is_unchanged`、手動 |
| P-23 | MP3、FLAC、AAC、ALAC、Ogg を再生できる | 手動 |
| P-24 | Opus を再生できる | 計画(段階 G) |

## エラー

| ID | 受入基準 | 確認 |
|---|---|---|
| P-30 | 再生できない理由を画面に表示する | `shows why a track could not be played` |
| P-31 | DRM の曲には専用の文言を出す | `protected_audio_gets_its_own_message` |
| P-32 | 再生操作の失敗で画面の処理が止まらない | `%s shows its failure instead of rejecting` |

## 応答性

| ID | 受入基準 | 確認 |
|---|---|---|
| P-40 | フォルダーの走査中も、再生の操作と表示が止まらない | 手動 |
| P-41 | 古い走査の結果で新しい結果を上書きしない | `ignores a slow answer to an older folder request`、`ignores a slow answer to an older album request` |
| P-42 | 状態の取得で再生キュー全体を送らない | `fetches the queue only when its version changes`、`the_version_follows_the_tracks_and_nothing_else` |
| P-44 | 一度開いたフォルダーは、もう一度開くとすぐに表示し、裏で読み直す | `shows a folder seen before at once and then reads it again`、`remembers only the most recent folders` |
| P-45 | 再読込は覚えていた結果を使わずに読み直す | `a refresh reads the folder again without the remembered result` |

P-43 は Android 版の文書プロバイダーに関する基準なので、Windows 版にはありません。

## 再生速度

| ID | 受入基準 | 確認 |
|---|---|---|
| P-60 | 再生速度は 0.8 倍から 2.0 倍まで、0.2 刻みで選べる | 計画(段階 E) |
| P-61 | 選んだ速度で再生する。音の高さは変えない | 計画(段階 E) |
| P-62 | 速度はアプリ全体で 1 つで、曲を替えても、起動し直しても保つ | 計画(段階 E) |
| P-63 | 保存された速度が選択肢にないときは、いちばん近い選択肢にする | 計画(段階 E) |
| P-64 | 1.0 倍以外のときは、再生バーに速度を表示する | 計画(段階 E) |
| P-65 | 速度を変えても、再生位置、シーク、曲の終わりの判定がずれない | 計画(段階 E) |

P-65 は Windows 版だけの基準です。Windows 版は再生位置を経過時間から計算するためです。

## OS のメディア操作

Android 版の「バックグラウンド」(P-50〜P-52)にあたる、Windows 版の基準です。

| ID | 受入基準 | 確認 |
|---|---|---|
| P-50 | Windows のメディア操作(音量の表示、ロック画面)に曲名とジャケットを出し、そこから再生、一時停止、前後の曲を操作できる | 計画(段階 F) |
| P-51 | キーボードのメディアキーと、ヘッドセットのボタンに応答する | 計画(段階 F) |
| P-53 | 既定の出力先が変わったとき(ヘッドホンの抜き差しなど)、新しい出力先で再生を続ける | 計画(段階 F) |

