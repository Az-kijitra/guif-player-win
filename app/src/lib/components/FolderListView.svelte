<script lang="ts">
  import { tick, untrack } from "svelte";
  import {
    focusNavItem,
    hasCommandModifier,
    nextListIndex,
    visibleItemCount,
  } from "../listNavigation";
  import { folderResumePoint } from "../albumResume";
  import { formatTime, playerState } from "../playerState.svelte";
  import type { AudioTrack, FolderItem } from "../types";

  let items = $derived(playerState.folderContent?.items ?? []);
  let audioTracks = $derived(
    items
      .filter((i) => !i.is_directory && i.track !== null)
      .map((i) => i.track as AudioTrack)
  );

  let resumePoint = $derived(
    folderResumePoint(audioTracks, playerState.albumResume, playerState.trackResumes)
  );

  let allSelected = $derived(
    audioTracks.length > 0 &&
      audioTracks.every((t) => playerState.isSelected(t.file_path))
  );

  let didInitialScroll = $state(false);
  let tableBodyEl = $state<HTMLDivElement | null>(null);
  let activeIndex = $state(0);
  let activeRow = $derived(Math.min(activeIndex, items.length - 1));
  let shownPath: string | null = null;

  // A new folder: start on the folder we came from, otherwise on the last track used
  $effect(() => {
    const content = playerState.folderContent;
    if (!content || content.current_path === shownPath) return;
    const cameFrom = shownPath;
    shownPath = content.current_path;

    untrack(() => {
      const wanted =
        cameFrom ?? playerState.lastSelectedPath ?? playerState.lastPlayedFilePath;
      activeIndex = Math.max(
        0,
        content.items.findIndex((i) => i.path === wanted)
      );
    });
    tick().then(() => {
      const focused = document.activeElement;
      if (focused === document.body || tableBodyEl?.contains(focused)) {
        focusNavItem(tableBodyEl, activeRow);
      }
    });
  });

  $effect(() => {
    if (
      !didInitialScroll &&
      playerState.lastPlayedFilePath &&
      items.length > 0 &&
      tableBodyEl
    ) {
      const targetEl = tableBodyEl.querySelector(
        '[data-last-played="true"]'
      ) as HTMLElement | null;
      if (targetEl) {
        targetEl.scrollIntoView({ block: "center" });
        didInitialScroll = true;
      }
    }
  });

  function toggleAll(e: Event) {
    e.stopPropagation();
    if (allSelected) {
      playerState.clearSelection();
    } else {
      playerState.selectAllTracks(audioTracks);
      playerState.showInspector = true;
    }
  }

  function onItemClick(e: MouseEvent, item: FolderItem) {
    if (item.is_directory) {
      playerState.navigateTo(item.path);
      return;
    }
    if (!item.track) return;

    if (e.shiftKey) {
      e.preventDefault();
      playerState.selectRange(item.track, audioTracks);
      playerState.showInspector = true;
    } else if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      playerState.toggleTrackSelection(item.track);
      playerState.showInspector = true;
    } else {
      playerState.selectSingleTrack(item.track);
      playerState.playTrackInList(item.track, audioTracks);
    }
  }

  function onRowKeydown(e: KeyboardEvent, item: FolderItem, index: number) {
    if (e.key === "Enter") {
      if (e.target !== e.currentTarget) return;
      if (item.is_directory) playerState.navigateTo(item.path);
      else if (item.track) {
        playerState.selectSingleTrack(item.track);
        playerState.playTrackInList(item.track, audioTracks);
      }
      return;
    }

    if (e.ctrlKey && e.key === " ") {
      e.preventDefault();
      if (item.track) {
        playerState.toggleTrackSelection(item.track);
        playerState.showInspector = true;
      }
      return;
    }
    if (hasCommandModifier(e)) return;

    const letter = e.shiftKey ? "" : e.key.toLowerCase();
    if (letter === "i" && item.track) {
      e.preventDefault();
      if (!playerState.isSelected(item.track.file_path)) {
        playerState.selectSingleTrack(item.track);
      }
      playerState.showInspector = true;
      return;
    }
    if (letter === "p" && item.track) {
      e.preventDefault();
      playerState.playlistModalTrack = item.track;
      return;
    }

    const next = nextListIndex(e.key, index, items.length, {
      pageSize: visibleItemCount(tableBodyEl),
    });
    if (next === null) return;
    e.preventDefault();
    if (e.shiftKey) {
      playerState.extendSelection(item.track, items[next].track, audioTracks);
    }
    activeIndex = next;
    focusNavItem(tableBodyEl, next);
  }

  function onCheckboxClick(e: MouseEvent, track: AudioTrack) {
    e.stopPropagation();
    if (e.shiftKey) {
      playerState.selectRange(track, audioTracks);
    } else {
      playerState.toggleTrackSelection(track);
    }
    playerState.showInspector = true;
  }

  function openInspectorForTrack(e: MouseEvent, track: AudioTrack) {
    e.stopPropagation();
    playerState.selectSingleTrack(track);
    playerState.showInspector = true;
  }

  function openAddToPlaylist(e: MouseEvent, track: AudioTrack) {
    e.stopPropagation();
    playerState.playlistModalTrack = track;
  }
</script>

<div class="folder-view">
  {#if items.length === 0}
    <div class="empty-state">
      {playerState.isLoadingFolder ? "読み込み中…" : "このフォルダにはアイテムがありません。"}
    </div>
  {:else}
    {#if audioTracks.length > 0}
      <div class="resume-banner">
        {#if resumePoint}
          <button
            class="resume-btn"
            title="このフォルダーを続きから再生"
            onclick={() => playerState.playAlbumResume(audioTracks)}
          >
            ▷ 続きから: {audioTracks[resumePoint.index].title}
            <span class="resume-at">{formatTime(resumePoint.positionMs)}</span>
          </button>
          <button
            class="restart-btn"
            title="このフォルダーを 1 曲目の最初から再生"
            onclick={() => playerState.playAlbumFromBeginning(audioTracks)}
          >
            最初から
          </button>
        {:else}
          <button
            class="resume-btn"
            title="このフォルダーを 1 曲目から再生"
            onclick={() => playerState.playAlbumFromBeginning(audioTracks)}
          >
            ▷ 最初から再生: {audioTracks[0].title}
          </button>
        {/if}
      </div>
    {/if}
    <div class="list-table">
      <div class="table-header">
        <span class="col-check">
          {#if audioTracks.length > 0}
            <input
              type="checkbox"
              checked={allSelected}
              onchange={toggleAll}
              title="すべての曲を選択 / 解除 (Ctrl+A)"
            />
          {/if}
        </span>
        <span class="col-num">#</span>
        <span class="col-title">タイトル / 名前</span>
        <span class="col-artist">アーティスト</span>
        <span class="col-album">アルバム</span>
        <span class="col-duration">時間</span>
        <span class="col-actions">操作</span>
      </div>

      <div class="table-body" bind:this={tableBodyEl}>
        {#each items as item, index}
          {@const isPlaying =
            playerState.playback.current_track?.file_path === item.path}
          {@const isLastPlayed =
            !playerState.playback.current_track &&
            playerState.lastPlayedFilePath === item.path}
          {@const isSelected = item.track
            ? playerState.isSelected(item.track.file_path)
            : false}
          {@const resume = item.track
            ? playerState.trackResumes[item.track.file_path]
            : null}

          <div
            class="table-row"
            class:is-dir={item.is_directory}
            class:is-playing={isPlaying}
            class:is-selected={isSelected}
            data-last-played={isLastPlayed ? "true" : undefined}
            onclick={(e) => onItemClick(e, item)}
            role="button"
            data-nav-item
            tabindex={index === activeRow ? 0 : -1}
            onfocus={() => (activeIndex = index)}
            onkeydown={(e) => onRowKeydown(e, item, index)}
          >
            <!-- Selection Checkbox -->
            <span class="col-check">
              {#if item.track}
                <input
                  type="checkbox"
                  checked={isSelected}
                  tabindex="-1"
                  onclick={(e) => onCheckboxClick(e, item.track!)}
                  title="選択（複数一括編集用） (Ctrl+Space)"
                />
              {/if}
            </span>

            <!-- Track Number / Folder Icon -->
            <span class="col-num">
              {#if item.is_directory}
                📁
              {:else if isPlaying}
                <span class="playing-anim"
                  >{playerState.playback.is_playing ? "▶" : "⏸"}</span
                >
              {:else if item.track?.track_number}
                {item.track.track_number}
              {:else}
                🎵
              {/if}
            </span>

            <!-- Title / Name -->
            <span class="col-title" title={item.name}>
              {item.is_directory ? item.name : (item.track?.title ?? item.name)}
              {#if item.is_directory && item.audio_count > 0}
                <span class="badge">{item.audio_count}曲</span>
              {/if}
              {#if isLastPlayed}
                <span class="badge" title="前回最後に再生していたファイル">前回再生</span>
              {/if}
              {#if resume?.is_completed}
                <span class="completed-check" title="再生完了">✓</span>
              {:else if resume && resume.position_ms > 0}
                <span class="resume-tag" title="途中から再開">
                  {formatTime(resume.position_ms)}
                </span>
              {/if}
            </span>

            <!-- Artist -->
            <span class="col-artist" title={item.track?.artist ?? ""}>
              {item.track?.artist ?? (item.is_directory ? "" : "-")}
            </span>

            <!-- Album -->
            <span class="col-album" title={item.track?.album ?? ""}>
              {item.track?.album ?? (item.is_directory ? "" : "-")}
            </span>

            <!-- Duration -->
            <span class="col-duration">
              {#if item.track}
                {formatTime(item.track.duration_ms)}
              {:else}
                -
              {/if}
            </span>

            <!-- Actions -->
            <span class="col-actions">
              {#if item.track}
                <button
                  class="action-btn"
                  tabindex="-1"
                  title="プロパティを編集 (I)"
                  onclick={(e) => openInspectorForTrack(e, item.track!)}
                >
                  🏷
                </button>
                <button
                  class="action-btn"
                  tabindex="-1"
                  title="プレイリストに追加 (P)"
                  onclick={(e) => openAddToPlaylist(e, item.track!)}
                >
                  +
                </button>
              {/if}
            </span>
          </div>
        {/each}
      </div>
    </div>
  {/if}
</div>

<style>
  .folder-view {
    flex: 1;
    overflow-y: auto;
    background: var(--nord0);
  }

  .resume-banner {
    display: flex;
    gap: 8px;
    padding: 8px 12px;
    background: var(--nord1);
    border-bottom: 1px solid var(--nord3);
  }

  .resume-btn,
  .restart-btn {
    padding: 4px 12px;
    border: 1px solid var(--nord8);
    border-radius: 3px;
    font-size: 12px;
    cursor: pointer;
  }

  .resume-btn {
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--nord0);
    background: var(--nord8);
    font-weight: bold;
  }

  .resume-at {
    margin-left: 6px;
    font-weight: normal;
  }

  .restart-btn {
    flex-shrink: 0;
    color: var(--nord8);
  }

  .empty-state {
    padding: 40px;
    text-align: center;
    color: var(--ui-text-muted);
  }

  .list-table {
    display: flex;
    flex-direction: column;
    min-width: 580px;
  }

  .table-header {
    display: flex;
    align-items: center;
    padding: 6px 12px;
    background: var(--nord1);
    color: var(--nord4);
    font-size: 11px;
    font-weight: bold;
    border-bottom: 1px solid var(--nord3);
    position: sticky;
    top: 0;
    z-index: 10;
  }

  .table-body {
    display: flex;
    flex-direction: column;
  }

  .table-row {
    display: flex;
    align-items: center;
    padding: 5px 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--nord3) 25%, transparent);
    cursor: pointer;
    font-size: 12px;
    color: var(--nord4);
    outline: none;
    scroll-margin-top: 30px;
  }

  .table-row:hover {
    background: var(--nord2);
    color: var(--nord6);
  }

  .table-row.is-selected {
    background: color-mix(in srgb, var(--nord9) 18%, transparent);
  }

  .table-row.is-playing {
    background: color-mix(in srgb, var(--nord8) 15%, transparent);
    color: var(--nord8);
    font-weight: bold;
  }

  .table-row.is-playing.is-selected {
    background: color-mix(in srgb, var(--nord8) 28%, transparent);
  }

  .table-row.is-dir {
    color: var(--nord9);
  }

  .col-check {
    width: 28px;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .col-check input[type="checkbox"] {
    cursor: pointer;
    accent-color: var(--nord8);
  }

  .col-num {
    width: 36px;
    text-align: center;
    color: var(--ui-text-muted);
  }

  .table-row.is-playing .col-num {
    color: var(--nord8);
  }

  .col-title {
    flex: 2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .col-artist {
    flex: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--nord4);
  }

  .col-album {
    flex: 1.2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ui-text-muted);
  }

  .col-duration {
    width: 56px;
    text-align: right;
    font-variant-numeric: tabular-nums;
    color: var(--ui-text-muted);
  }

  .col-actions {
    width: 64px;
    display: flex;
    justify-content: flex-end;
    gap: 4px;
  }

  .badge {
    font-size: 10px;
    background: var(--nord2);
    color: var(--nord8);
    padding: 1px 5px;
    border-radius: 3px;
    border: 1px solid var(--nord3);
  }

  .completed-check {
    font-size: 11px;
    color: var(--nord14);
    font-weight: bold;
  }

  .resume-tag {
    font-size: 10px;
    background: color-mix(in srgb, var(--nord13) 20%, transparent);
    color: var(--nord13);
    padding: 0 4px;
    border-radius: 2px;
  }

  .action-btn {
    padding: 1px 5px;
    background: var(--nord2);
    color: var(--nord4);
    border: 1px solid var(--nord3);
    border-radius: 2px;
    font-size: 11px;
  }

  .action-btn:hover {
    background: var(--nord8);
    color: var(--nord0);
    border-color: var(--nord8);
  }
</style>
