<script lang="ts">
  // The tracks of the album opened in the album view: play it from where it was left off,
  // from the start, shuffled, or from any track, without leaving the album view.
  import { tick } from "svelte";
  import { api } from "../api";
  import { folderResumePoint } from "../albumResume";
  import { focusNavItem, hasCommandModifier, nextListIndex, visibleItemCount } from "../listNavigation";
  import { formatTime, playerState } from "../playerState.svelte";
  import type { AudioTrack, OpenedAlbum } from "../types";

  let { opened }: { opened: OpenedAlbum } = $props();

  let tracks = $derived(opened.tracks);
  let resumePoint = $derived(folderResumePoint(tracks, opened.resume, playerState.trackResumes));
  let totalMs = $derived(tracks.reduce((sum, t) => sum + t.duration_ms, 0));
  let playingPath = $derived(playerState.playback.current_track?.file_path);

  let art = $state<string | null>(null);
  let listEl = $state<HTMLDivElement | null>(null);
  let activeIndex = $state(0);

  // Only the path and the artwork version: the opened album is replaced when its resume
  // point changes, and that must not reload (and blink) the artwork.
  let artPath = $derived(opened.album.has_artwork ? opened.album.first_track_path : null);
  let artVersion = $derived(playerState.artworkVersion); // a new artwork from the properties
  let artShownFor: string | null = null;

  $effect(() => {
    const path = artPath;
    void artVersion;
    if (path !== artShownFor) {
      // Another album: not its artwork while the new one loads; the same one is kept
      art = null;
      artShownFor = null;
    }
    if (!path) return;
    let current = true;
    api
      .getArtwork(path)
      .then((a) => {
        if (current) {
          art = a;
          artShownFor = path;
        }
      })
      .catch(() => {});
    return () => {
      current = false;
    };
  });

  // The list takes the focus when the album opens, so the keys work at once
  $effect(() => {
    if (tracks.length > 0 && listEl) {
      activeIndex = 0;
      tick().then(() => focusNavItem(listEl, 0));
    }
  });

  // Like the folder view: a track played is the one selected, so the properties show it
  function play(track: AudioTrack) {
    playerState.selectSingleTrack(track);
    playerState.playTrackInList(track, tracks);
  }

  function onRowClick(e: MouseEvent, track: AudioTrack) {
    if (e.shiftKey) {
      e.preventDefault();
      playerState.selectRange(track, tracks);
      playerState.showInspector = true;
    } else if (e.ctrlKey || e.metaKey) {
      e.preventDefault();
      playerState.toggleTrackSelection(track);
      playerState.showInspector = true;
    } else {
      play(track);
    }
  }

  function onRowKeydown(e: KeyboardEvent, track: AudioTrack, index: number) {
    if (e.key === "Enter") {
      e.preventDefault();
      play(track);
      return;
    }
    if (e.ctrlKey && e.key === " ") {
      e.preventDefault();
      playerState.toggleTrackSelection(track);
      playerState.showInspector = true;
      return;
    }
    if (hasCommandModifier(e) || !listEl) return;
    if (!e.shiftKey && e.key.toLowerCase() === "i") {
      e.preventDefault();
      if (!playerState.isSelected(track.file_path)) {
        playerState.selectSingleTrack(track);
      }
      playerState.showInspector = true;
      return;
    }
    const next = nextListIndex(e.key, index, tracks.length, { pageSize: visibleItemCount(listEl) });
    if (next === null) return;
    e.preventDefault();
    if (e.shiftKey) {
      playerState.extendSelection(track, tracks[next], tracks);
    }
    activeIndex = next;
    focusNavItem(listEl, next);
  }
</script>

<div class="album-detail">
  <div class="detail-header">
    <button class="back-btn" title="アルバムの一覧へ戻る (Backspace / Esc)" onclick={() => playerState.closeAlbum()}>
      ← アルバム一覧
    </button>
  </div>

  <div class="album-head">
    <div class="art">
      {#if art}
        <img src={art} alt={opened.album.album_name} />
      {:else}
        <span class="disc-icon">💿</span>
      {/if}
    </div>
    <div class="album-meta">
      <div class="album-name">{opened.album.album_name}</div>
      <div class="album-artist">{opened.album.artist_name}</div>
      <div class="album-stats">{tracks.length} 曲 ・ {formatTime(totalMs)}</div>
      <div class="actions">
        {#if resumePoint}
          <button
            class="primary"
            title="このアルバムを続きから再生"
            onclick={() => playerState.playAlbumResume(tracks, opened.resume)}
          >
            ▷ 続きから: {tracks[resumePoint.index].title}
            <span class="at">{formatTime(resumePoint.positionMs)}</span>
          </button>
          <button title="1 曲目の最初から再生" onclick={() => playerState.playAlbumFromBeginning(tracks)}>
            最初から
          </button>
        {:else}
          <button
            class="primary"
            title="1 曲目から再生"
            onclick={() => playerState.playAlbumFromBeginning(tracks)}
          >
            ▶ 再生
          </button>
        {/if}
        <button title="ばらばらの順に再生" onclick={() => playerState.playAll(tracks, true)}>🔀 シャッフル</button>
        <button
          title="フォルダービューでこのフォルダーを開く"
          onclick={() => playerState.openAlbumInFolderView(opened.album)}
        >
          📂 フォルダーで開く
        </button>
      </div>
    </div>
  </div>

  <div class="track-list" bind:this={listEl}>
    {#each tracks as track, index}
      {@const resume = playerState.trackResumes[track.file_path]}
      <div
        class="track-row"
        class:is-playing={track.file_path === playingPath}
        class:is-selected={playerState.isSelected(track.file_path)}
        role="button"
        data-nav-item
        tabindex={index === activeIndex ? 0 : -1}
        onclick={(e) => onRowClick(e, track)}
        onfocus={() => (activeIndex = index)}
        onkeydown={(e) => onRowKeydown(e, track, index)}
      >
        <span class="num">{track.file_path === playingPath ? "♪" : (track.track_number ?? index + 1)}</span>
        <span class="title">
          {track.title}
          {#if resume?.is_completed}
            <span class="done" title="最後まで聴いた曲">✓</span>
          {:else if resume && resume.position_ms > 0}
            <span class="resume-tag" title="途中から再開">{formatTime(resume.position_ms)}</span>
          {/if}
        </span>
        <span class="artist">{track.artist}</span>
        <span class="duration">{formatTime(track.duration_ms)}</span>
      </div>
    {/each}
  </div>
</div>

<style>
  .album-detail {
    display: flex;
    flex-direction: column;
    min-height: 100%;
  }

  .detail-header {
    margin-bottom: 12px;
  }

  .back-btn,
  .actions button {
    padding: 4px 12px;
    color: var(--nord8);
    background: transparent;
    border: 1px solid var(--nord8);
    border-radius: 3px;
    font-size: 12px;
    font-family: inherit;
    cursor: pointer;
  }

  .back-btn {
    color: var(--nord4);
    border-color: var(--nord3);
  }

  .album-head {
    display: flex;
    gap: 20px;
    margin-bottom: 16px;
  }

  .art {
    flex-shrink: 0;
    width: 160px;
    height: 160px;
    display: flex;
    align-items: center;
    justify-content: center;
    background: var(--nord1);
    border: 1px solid var(--nord3);
    border-radius: 4px;
    overflow: hidden;
  }

  .art img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .disc-icon {
    font-size: 48px;
    opacity: 0.6;
  }

  .album-meta {
    min-width: 0;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .album-name {
    font-size: 20px;
    font-weight: bold;
    color: var(--nord6);
  }

  .album-artist {
    color: var(--nord4);
  }

  .album-stats {
    font-size: 12px;
    color: var(--ui-text-muted);
  }

  .actions {
    display: flex;
    flex-wrap: wrap;
    gap: 8px;
    margin-top: auto;
    padding-top: 8px;
  }

  .actions .primary {
    max-width: 320px;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--nord0);
    background: var(--nord8);
    font-weight: bold;
  }

  .at {
    margin-left: 6px;
    font-weight: normal;
  }

  .track-list {
    border-top: 1px solid var(--nord3);
  }

  .track-row {
    display: grid;
    grid-template-columns: 40px minmax(0, 2fr) minmax(0, 1fr) 60px;
    align-items: center;
    gap: 8px;
    padding: 7px 8px;
    border-bottom: 1px solid var(--nord1);
    cursor: pointer;
  }

  .track-row:hover {
    background: var(--nord1);
  }

  .track-row.is-selected {
    background: color-mix(in srgb, var(--nord9) 18%, transparent);
  }

  .track-row:focus {
    outline: 1px solid var(--nord8);
    outline-offset: -1px;
  }

  .track-row.is-playing {
    color: var(--nord8);
    font-weight: bold;
  }

  .num,
  .duration {
    color: var(--ui-text-muted);
    font-size: 12px;
  }

  .num {
    text-align: right;
  }

  .duration {
    text-align: right;
  }

  .title,
  .artist {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .artist {
    color: var(--nord4);
    font-size: 12px;
  }

  .done {
    margin-left: 4px;
    color: var(--nord14);
  }

  .resume-tag {
    margin-left: 6px;
    padding: 0 4px;
    font-size: 11px;
    font-weight: normal;
    color: var(--nord13);
    background: color-mix(in srgb, var(--nord13) 20%, transparent);
    border-radius: 2px;
  }
</style>
