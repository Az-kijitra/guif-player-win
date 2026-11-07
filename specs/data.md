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

## 再生位置

| ID | 受入基準 | 確認 |
|---|---|---|
| D-20 | 再生中は 3 秒ごとに位置を保存する | 手動 |
| D-21 | 途中まで聴いた曲はその位置から、聴き終えた曲は先頭から再生する | 手動 |
| D-22 | 起動すると前回のフォルダーを開く | 手動 |
