# データの保存と再生位置

## 保存先

| ID | 受入基準 | 確認 |
|---|---|---|
| D-1 | 新しい環境では `dev.guif.player` フォルダーを使う | `a_fresh_install_uses_the_new_folder` |
| D-2 | 旧フォルダーのデータベースをデータごと移動する | `the_legacy_database_moves_with_its_data` |
| D-3 | 旧フォルダーのほかのファイルに触れない | `other_files_in_the_legacy_folder_are_left_alone` |
| D-4 | 新しいデータベースがあれば上書きしない | `an_existing_new_database_is_never_overwritten` |
| D-5 | 移動できないときは旧データベースを使い続け、次の起動でまた試す | `a_locked_legacy_database_stays_in_use` |

## スキーマ

| ID | 受入基準 | 確認 |
|---|---|---|
| D-10 | 新しいデータベースは最新のスキーマになる | `a_new_database_gets_the_latest_schema` |
| D-11 | 版のない旧データベースのデータが残る | `a_database_from_before_versioning_keeps_its_data` |
| D-12 | 失敗した手順は取り消し、それまでの手順は残る | `a_failing_migration_rolls_back_and_keeps_the_earlier_steps` |
| D-13 | 古いデータベースに新しい手順を適用できる | `a_later_migration_runs_on_an_older_database` |
| D-14 | アプリより新しいデータベースは開かず、変更しない | `a_database_from_a_newer_version_is_refused_and_left_alone` |
| D-15 | データベースを開けないときは理由を表示する | 手動 |
| D-16 | 再生位置の保存の一部が失敗したら、すべて取り消す | `a_failed_track_resume_write_rolls_back_the_album_resume_write` |

## 再生位置

| ID | 受入基準 | 確認 |
|---|---|---|
| D-20 | 再生中は 3 秒ごとに位置を保存する | 手動 |
| D-21 | 途中まで聴いた曲はその位置から、聴き終えた曲は先頭から再生する | `a partly played track starts from its position`、`a completed track starts from the beginning` |
| D-22 | 起動すると前回のフォルダーを開く | 手動 |
| D-23 | 残り 3 秒以内まで聴いた曲を、聴き終えた曲とする | `a_track_is_completed_within_the_last_three_seconds` |
| D-24 | アルバムの「続きから」は、最後に聴いていた曲の位置から再生する | `album resumes at the position of an unfinished track`、`plays the folder from where it was left off` |
| D-25 | 最後の曲を聴き終えていれば次の曲の先頭から、アルバムの最後なら 1 曲目から再生する | `album resumes at the next track after a completed track`、`album starts over after its last track is completed` |
| D-26 | 曲が切り替わると、前の曲の最後の位置をその曲のアルバムに保存する | `leaving_a_track_records_its_position_under_its_own_album` |
| D-27 | 再生を始めて 1 秒未満の位置は保存しない | `the_first_second_of_a_track_is_not_recorded` |
| D-28 | 保存された曲がアルバムになければ 1 曲目から再生する | `album starts over when the saved track is gone`、`offers nothing to resume in a folder never played or whose track is gone` |
| S-12 | フォルダーの一覧に、続きから再生する曲を表示する。「最初から」でアルバムの 1 曲目の最初から再生できる | `folder shows the track its album resumes from`、`plays the folder from the first track at its start` |
| D-29 | 一時停止したときも位置を保存する | `pausing_records_the_position` |

S-12 は Android 版の storage.md の基準です。Windows 版はフォルダービューにだけ表示し、アルバムビューには表示しません(アルバムビューはフォルダーを移動する画面ではないため)。

D-29 は Windows 版で番号を付けた基準です。Android 版も再生が止まると位置を記録しています。

## プレイリスト

| ID | 受入基準 | 確認 |
|---|---|---|
| D-30 | プレイリストを削除すると、その曲の登録だけが消え、音楽ファイルと他のプレイリストは残る | `deleting_a_playlist_removes_its_tracks_and_keeps_the_others` |
| D-31 | 開いているプレイリストを削除すると一覧に戻る | `deleting the open playlist closes it` |
| D-32 | 同じ名前のプレイリストは作らず、理由を表示する | `creating_a_playlist_with_an_existing_name_is_refused`、`a refused name adds nothing to another playlist` |
| D-33 | 見つからない曲があれば、その数を表示する | `missing_files_are_reported`、`says how many tracks of a playlist are missing` |

## しおり

| ID | 受入基準 | 確認 |
|---|---|---|
| D-40 | しおりは 1 つの曲に 1 つ。同じ曲で挟み直すと位置を上書きする | `a_track_has_one_bookmark_and_bookmarking_it_again_moves_it` |
| D-41 | 複数の曲にしおりを挟め、一覧は新しく挟んだ順に並ぶ | `bookmarks_of_several_tracks_are_listed_newest_first` |
| D-42 | 再生画面で、今の位置にしおりを挟み、挟んだ位置を知らせる | `bookmarking_the_current_track_saves_where_it_is`、`bookmarking_with_nothing_playing_is_refused`、`bookmarking a track lists it and says where` |
| D-43 | しおりを削除でき、ほかのしおりは残る | `deleting_a_bookmark_leaves_the_others`、`deleting a bookmark removes it from the list` |
| D-44 | 一覧のしおりを選ぶと、その位置から再生し、アルバムの残りの曲に続ける | `playing a bookmark starts its track at the bookmark and keeps the album queued` |
| D-45 | 曲が見つからなければ理由を表示し、しおりは残す | `playing a bookmark whose track is gone shows an error` |
| D-46 | 古いデータベースを更新しても、データが残り、しおりを使える | `upgrading_to_bookmarks_keeps_the_data` |

