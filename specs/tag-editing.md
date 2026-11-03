# タグの保存

音楽ファイルは利用者の財産です。**タグの保存で音声と、頼まれていない情報を変えてはいけません。**

## ファイルの保護

| ID | 受入基準 | 確認 |
|---|---|---|
| T-1 | 保存に失敗したファイルは元のまま残る | `a_failed_file_stays_unchanged_and_the_others_are_still_written` |
| T-2 | 読み取り専用のファイルは保存しない | `a_read_only_file_is_refused` |
| T-3 | ほかのアプリが開いているファイルは変更しない | `a_file_held_open_by_another_program_stays_unchanged` |
| T-4 | 作成日時が変わらない | `the_creation_date_is_kept` |
| T-5 | 一時ファイルを残さない。異常終了で残ったものは一覧に出さず、次の保存で片付ける | `a_working_copy_left_by_a_crash_is_replaced_and_not_listed` |
| T-6 | 置き換える前に、音声、再生時間、タグを確かめる | `the_final_check_notices_changed_audio`、`the_final_check_notices_a_changed_length`、`the_final_check_notices_tags_that_were_not_written` |
| T-7 | 対応していない画像なら、どのファイルにも触れない | `an_unsupported_image_stops_before_any_file_is_touched` |

## 音声の保持

| ID | 受入基準 | 確認 |
|---|---|---|
| T-10 | WAV、AIFF、MP3、FLAC の音声が変わらない | `writes_tags_and_keeps_the_audio_in_wav`、`writes_tags_and_keeps_the_audio_in_aiff`、`writes_tags_and_keeps_the_audio_in_mp3`、`writes_tags_and_keeps_the_audio_in_flac` |
| T-11 | FLAC はメタデータの構成によらず音声が変わらない | `flac_audio_survives_every_metadata_layout` |
| T-12 | MP3 と M4A の実ファイルで音声が変わらない | 手動(実ファイルのコピーで全編のデコード結果を比較) |

## 情報の保持

| ID | 受入基準 | 確認 |
|---|---|---|
| T-20 | MP3 で、変更していないフレームと APE タグが残る | `mp3_keeps_what_the_request_did_not_touch` |
| T-21 | MP3 の ID3 の版を保つ | `mp3_keeps_its_id3_version` |
| T-22 | v2.3 に残せないフレームがあれば、消さずに v2.4 にする | `mp3_moves_to_v24_rather_than_lose_a_frame` |
| T-23 | 言語コードが不正なコメントがあっても保存できる | `mp3_with_an_invalid_comment_language_can_be_saved` |
| T-24 | FLAC の既存のコメントが残る | `existing_flac_comments_are_kept` |
| T-25 | M4A の実ファイルで、変更していない情報が残る | 手動 |

## 編集の内容

| ID | 受入基準 | 確認 |
|---|---|---|
| T-30 | 選んだ順に連番を振り、番号を消せる | `numbers_tracks_in_the_order_given_and_clears_numbers` |
| T-31 | ファイル名をタイトルにできる | `uses_the_file_name_as_the_title` |
| T-32 | ジャケットを置き換え、消せる | `sets_and_clears_artwork`、`mp3_artwork_is_replaced_and_cleared` |

## 結果の表示

| ID | 受入基準 | 確認 |
|---|---|---|
| T-40 | 一部が失敗したら、保存できた数と、失敗した曲と理由を表示する | `reports both counts and names the failed file when some were saved` |
| T-41 | すべて失敗したら入力を残し、やり直せる | `says nothing changed when every file failed`、手動 |
| T-42 | 再生中の曲を保存しても、再生と位置が続く | 手動 |
