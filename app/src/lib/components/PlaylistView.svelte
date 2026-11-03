<script lang="ts">
  import { tick } from "svelte";
  import {
    focusNavItem,
    hasCommandModifier,
    nextListIndex,
    visibleItemCount,
  } from "../listNavigation";
  import { formatTime, playerState } from "../playerState.svelte";
  import type { AudioTrack, Playlist } from "../types";

  let newPlaylistName = $state("");
  let sidebarListEl = $state<HTMLDivElement | null>(null);
  let tracksEl = $state<HTMLDivElement | null>(null);
  let activePlaylistIndex = $state(0);
  let activeTrackIndex = $state(0);
  let activePlaylistRow = $derived(
    Math.min(activePlaylistIndex, playerState.playlists.length - 1)
  );
  let activeTrackRow = $derived(
    Math.min(activeTrackIndex, playerState.playlistTracks.length - 1)
  );

  // The list loads after the view opens, so wait for its rows
  $effect(() => {
    if (playerState.playlists.length > 0 && sidebarListEl) {
      tick().then(() => {
        if (document.activeElement === document.body) {
          focusNavItem(sidebarListEl, activePlaylistRow);
        }
      });
    }
  });

  function confirmDeletePlaylist(p: Playlist) {
    return confirm(`プレイリスト「${p.name}」を削除しますか？`);
  }

  async function onPlaylistKeydown(e: KeyboardEvent, p: Playlist, index: number) {
    if (e.key === "Enter") {
      if (e.target !== e.currentTarget) return;
      await playerState.selectPlaylist(p.id);
      activeTrackIndex = 0;
      await tick();
      focusNavItem(tracksEl, 0);
      return;
    }
    if (hasCommandModifier(e) || e.shiftKey) return;

    if (e.key === "Delete") {
      e.preventDefault();
      if (confirmDeletePlaylist(p)) {
        await playerState.deletePlaylist(p.id);
        await tick();
        focusNavItem(sidebarListEl, activePlaylistRow);
      }
      return;
    }
    if (e.key === "ArrowRight") {
      if (playerState.activePlaylistId === p.id) {
        e.preventDefault();
        focusNavItem(tracksEl, activeTrackRow);
      }
      return;
    }

    const next = nextListIndex(e.key, index, playerState.playlists.length, {
      pageSize: visibleItemCount(sidebarListEl),
    });
    if (next === null) return;
    e.preventDefault();
    activePlaylistIndex = next;
    focusNavItem(sidebarListEl, next);
  }

  async function onTrackKeydown(e: KeyboardEvent, track: AudioTrack, index: number) {
    const tracks = playerState.playlistTracks;
    if (e.key === "Enter") {
      if (e.target !== e.currentTarget) return;
      playerState.selectSingleTrack(track);
      playerState.playTrackInList(track, tracks);
      return;
    }

    if (e.ctrlKey && e.key === " ") {
      e.preventDefault();
      playerState.toggleTrackSelection(track);
      playerState.showInspector = true;
      return;
    }
    if (hasCommandModifier(e)) return;

    const letter = e.shiftKey ? "" : e.key.toLowerCase();
    if (letter === "i") {
      e.preventDefault();
      if (!playerState.isSelected(track.file_path)) {
        playerState.selectSingleTrack(track);
      }
      playerState.showInspector = true;
      return;
    }
    if (letter === "p") {
      e.preventDefault();
      playerState.playlistModalTrack = track;
      return;
    }
    if (e.key === "Delete" && !e.shiftKey) {
      e.preventDefault();
      await playerState.removeTrackFromPlaylist(
        playerState.activePlaylistId!,
        track.file_path
      );
      await tick();
      focusNavItem(tracksEl, activeTrackRow);
      if (playerState.playlistTracks.length === 0) {
        focusNavItem(sidebarListEl, activePlaylistRow);
      }
      return;
    }
    if (e.key === "ArrowLeft" && !e.shiftKey) {
      e.preventDefault();
      focusNavItem(sidebarListEl, activePlaylistRow);
      return;
    }

    const next = nextListIndex(e.key, index, tracks.length, {
      pageSize: visibleItemCount(tracksEl),
    });
    if (next === null) return;
    e.preventDefault();
    if (e.shiftKey) {
      playerState.extendSelection(track, tracks[next], tracks);
    }
    activeTrackIndex = next;
    focusNavItem(tracksEl, next);
  }

  async function handleCreatePlaylist(e: SubmitEvent) {
    e.preventDefault();
    if (!newPlaylistName.trim()) return;
    await playerState.createPlaylist(newPlaylistName.trim());
    newPlaylistName = "";
  }

  function onTrackClick(e: MouseEvent, track: AudioTrack) {
    if (e.shiftKey) {
      e.preventDefault();
      playerState.selectRange(track, playerState.playlistTracks);
      playerState.showInspector = true;
    } else if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      playerState.toggleTrackSelection(track);
      playerState.showInspector = true;
    } else {
      playerState.selectSingleTrack(track);
      playerState.playTrackInList(track, playerState.playlistTracks);
    }
  }
</script>

<div class="playlist-view">
  <!-- Playlists Sidebar -->
  <aside class="sidebar">
    <div class="sidebar-header">
      <span class="header-title">プレイリスト</span>
      <span class="count">{playerState.playlists.length}件</span>
    </div>

    <!-- Create Form -->
    <form class="create-form" onsubmit={handleCreatePlaylist}>
      <input
        type="text"
        placeholder="新規プレイリスト名..."
        bind:value={newPlaylistName}
        class="name-input"
      />
      <button type="submit" class="create-btn" title="作成">+</button>
    </form>

    <div class="playlist-list" bind:this={sidebarListEl}>
      {#each playerState.playlists as p, index}
        {@const isSelected = playerState.activePlaylistId === p.id}
        <div
          class="playlist-item"
          class:selected={isSelected}
          role="button"
          data-nav-item
          tabindex={index === activePlaylistRow ? 0 : -1}
          onclick={() => playerState.selectPlaylist(p.id)}
          onfocus={() => (activePlaylistIndex = index)}
          onkeydown={(e) => onPlaylistKeydown(e, p, index)}
        >
          <span class="item-name">{p.name}</span>
          <span class="item-count">{p.track_count}曲</span>
          <button
            class="delete-btn"
            tabindex="-1"
            title="プレイリストを削除 (Delete)"
            onclick={(e) => {
              e.stopPropagation();
              if (confirmDeletePlaylist(p)) {
                playerState.deletePlaylist(p.id);
              }
            }}
          >
            ×
          </button>
        </div>
      {/each}
    </div>
  </aside>

  <!-- Playlist Tracks Main Pane -->
  <main class="main-content">
    {#if playerState.activePlaylistId === null}
      <div class="empty-state">
        左側のリストからプレイリストを選択してください。
      </div>
    {:else}
      {@const currentPlaylist = playerState.playlists.find(
        (p) => p.id === playerState.activePlaylistId
      )}
      <div class="content-header">
        <div class="playlist-info">
          <h2>{currentPlaylist?.name ?? "プレイリスト"}</h2>
          <span class="track-summary">{playerState.playlistTracks.length} 曲</span>
        </div>
        {#if playerState.playlistTracks.length > 0}
          <button
            class="play-all-btn"
            onclick={() =>
              playerState.playTrackInList(
                playerState.playlistTracks[0],
                playerState.playlistTracks
              )}
          >
            ▶ すべて再生
          </button>
        {/if}
      </div>

      {#if playerState.playlistTracks.length === 0}
        <div class="empty-tracks">
          このプレイリストには曲がありません。<br />
          [📁 フォルダ] ビューの曲の「+」ボタンから曲を追加できます。
        </div>
      {:else}
        <div class="tracks-table">
          <div class="table-header">
            <span class="col-check">
              <input
                type="checkbox"
                checked={playerState.playlistTracks.length > 0 &&
                  playerState.playlistTracks.every((t) =>
                    playerState.isSelected(t.file_path)
                  )}
                onchange={() => {
                  const allSel =
                    playerState.playlistTracks.length > 0 &&
                    playerState.playlistTracks.every((t) =>
                      playerState.isSelected(t.file_path)
                    );
                  if (allSel) playerState.clearSelection();
                  else {
                    playerState.selectAllTracks(playerState.playlistTracks);
                    playerState.showInspector = true;
                  }
                }}
              />
            </span>
            <span class="col-num">#</span>
            <span class="col-title">タイトル</span>
            <span class="col-artist">アーティスト</span>
            <span class="col-album">アルバム</span>
            <span class="col-duration">時間</span>
            <span class="col-del">削除</span>
          </div>

          <div class="table-body" bind:this={tracksEl}>
            {#each playerState.playlistTracks as track, idx}
              {@const isPlaying =
                playerState.playback.current_track?.file_path === track.file_path}
              {@const isSelected = playerState.isSelected(track.file_path)}
              <div
                class="table-row"
                class:is-playing={isPlaying}
                class:is-selected={isSelected}
                role="button"
                data-nav-item
                tabindex={idx === activeTrackRow ? 0 : -1}
                onclick={(e) => onTrackClick(e, track)}
                onfocus={() => (activeTrackIndex = idx)}
                onkeydown={(e) => onTrackKeydown(e, track, idx)}
              >
                <span class="col-check">
                  <input
                    type="checkbox"
                    checked={isSelected}
                    tabindex="-1"
                    title="選択 (Ctrl+Space)"
                    onclick={(e) => {
                      e.stopPropagation();
                      if (e.shiftKey) {
                        playerState.selectRange(track, playerState.playlistTracks);
                      } else {
                        playerState.toggleTrackSelection(track);
                      }
                      playerState.showInspector = true;
                    }}
                  />
                </span>
                <span class="col-num">{idx + 1}</span>
                <span class="col-title">{track.title}</span>
                <span class="col-artist">{track.artist}</span>
                <span class="col-album">{track.album}</span>
                <span class="col-duration">{formatTime(track.duration_ms)}</span>
                <span class="col-del">
                  <button
                    class="del-track-btn"
                    tabindex="-1"
                    title="このプレイリストから削除 (Delete)"
                    onclick={(e) => {
                      e.stopPropagation();
                      playerState.removeTrackFromPlaylist(
                        playerState.activePlaylistId!,
                        track.file_path
                      );
                    }}
                  >
                    ×
                  </button>
                </span>
              </div>
            {/each}
          </div>
        </div>
      {/if}
    {/if}
  </main>
</div>

<style>
  .playlist-view {
    flex: 1;
    display: flex;
    overflow: hidden;
    background: var(--nord0);
  }

  .sidebar {
    width: 240px;
    background: var(--nord1);
    border-right: 1px solid var(--nord3);
    display: flex;
    flex-direction: column;
  }

  .sidebar-header {
    padding: 10px 12px;
    display: flex;
    justify-content: space-between;
    align-items: center;
    border-bottom: 1px solid var(--nord3);
    font-size: 12px;
    font-weight: bold;
    color: var(--nord4);
  }

  .count {
    color: var(--nord3);
    font-size: 11px;
  }

  .create-form {
    padding: 8px 12px;
    display: flex;
    gap: 4px;
    border-bottom: 1px solid var(--nord3);
  }

  .name-input {
    flex: 1;
    font-size: 11px;
    padding: 4px 6px;
    background: var(--nord0);
  }

  .create-btn {
    padding: 0 10px;
    background: var(--nord8);
    color: var(--nord0);
    font-weight: bold;
    border-radius: 2px;
  }

  .playlist-list {
    flex: 1;
    overflow-y: auto;
  }

  .playlist-item {
    padding: 8px 12px;
    display: flex;
    align-items: center;
    gap: 6px;
    cursor: pointer;
    font-size: 12px;
    border-bottom: 1px solid color-mix(in srgb, var(--nord3) 25%, transparent);
  }

  .playlist-item:hover {
    background: var(--nord2);
  }

  .playlist-item.selected {
    background: var(--nord2);
    border-left: 3px solid var(--nord8);
    color: var(--nord8);
    font-weight: bold;
  }

  .item-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .item-count {
    font-size: 10px;
    color: var(--nord3);
  }

  .delete-btn {
    padding: 0 4px;
    color: var(--nord3);
    font-size: 14px;
  }

  .delete-btn:hover {
    color: var(--nord11);
  }

  .main-content {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow-y: auto;
  }

  .content-header {
    padding: 16px 20px;
    display: flex;
    align-items: center;
    justify-content: space-between;
    border-bottom: 1px solid var(--nord3);
    background: var(--nord1);
  }

  .content-header h2 {
    margin: 0;
    font-size: 18px;
    color: var(--nord6);
  }

  .track-summary {
    font-size: 12px;
    color: var(--nord3);
  }

  .play-all-btn {
    padding: 6px 14px;
    background: var(--nord8);
    color: var(--nord0);
    font-weight: bold;
    border-radius: 3px;
    font-size: 12px;
  }

  .empty-state, .empty-tracks {
    padding: 60px 20px;
    text-align: center;
    color: var(--nord3);
    line-height: 1.6;
  }

  .tracks-table {
    display: flex;
    flex-direction: column;
  }

  .table-header {
    display: flex;
    padding: 6px 16px;
    background: var(--nord1);
    font-size: 11px;
    font-weight: bold;
    color: var(--nord4);
    border-bottom: 1px solid var(--nord3);
  }

  .table-row {
    display: flex;
    padding: 6px 16px;
    border-bottom: 1px solid color-mix(in srgb, var(--nord3) 25%, transparent);
    cursor: pointer;
    font-size: 12px;
    align-items: center;
  }

  .table-row:hover {
    background: var(--nord2);
  }

  .table-row.is-selected {
    background: color-mix(in srgb, var(--nord9) 18%, transparent);
  }

  .table-row.is-playing {
    color: var(--nord8);
    font-weight: bold;
    background: color-mix(in srgb, var(--nord8) 15%, transparent);
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
    width: 32px;
    color: var(--nord3);
  }

  .col-title {
    flex: 2;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
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
    color: var(--nord3);
  }

  .col-duration {
    width: 60px;
    text-align: right;
    color: var(--nord3);
  }

  .col-del {
    width: 36px;
    text-align: center;
  }

  .del-track-btn {
    color: var(--nord3);
    font-size: 14px;
  }

  .del-track-btn:hover {
    color: var(--nord11);
  }
</style>
