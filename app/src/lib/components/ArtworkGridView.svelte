<script lang="ts">
  import { tick } from "svelte";
  import { api } from "../api";
  import {
    focusNavItem,
    hasCommandModifier,
    nextListIndex,
    visibleItemCount,
  } from "../listNavigation";
  import { playerState } from "../playerState.svelte";
  import type { AlbumSummary } from "../types";

  let albums = $derived(playerState.albums);
  let artworkCache = $state<Record<string, string | null>>({});
  let gridEl = $state<HTMLDivElement | null>(null);
  let activeIndex = $state(0);
  let activeCard = $derived(Math.min(activeIndex, albums.length - 1));

  $effect(() => {
    if (albums.length > 0 && gridEl) {
      tick().then(() => {
        if (document.activeElement === document.body) {
          focusNavItem(gridEl, activeCard);
        }
      });
    }
  });

  function onCardKeydown(e: KeyboardEvent, album: AlbumSummary, index: number) {
    if (e.key === "Enter") {
      onAlbumClick(album);
      return;
    }
    if (hasCommandModifier(e) || e.shiftKey || !gridEl) return;

    const columns = getComputedStyle(gridEl).gridTemplateColumns.split(" ").length;
    const next = nextListIndex(e.key, index, albums.length, {
      columns,
      pageSize: columns * visibleItemCount(gridEl),
    });
    if (next === null) return;
    e.preventDefault();
    activeIndex = next;
    focusNavItem(gridEl, next);
  }

  async function loadArtworks(list: AlbumSummary[]) {
    for (const a of list) {
      if (a.has_artwork && !artworkCache[a.first_track_path]) {
        try {
          const art = await api.getArtwork(a.first_track_path);
          artworkCache[a.first_track_path] = art;
        } catch {
          artworkCache[a.first_track_path] = null;
        }
      }
    }
  }

  $effect(() => {
    if (albums.length > 0) {
      loadArtworks(albums);
    }
  });

  function onAlbumClick(album: AlbumSummary) {
    playerState.openAlbumViewItem(album);
  }
</script>

<div class="artwork-grid-view">
  {#if albums.length === 0 && playerState.isLoadingAlbums}
    <div class="empty-state">読み込み中…</div>
  {:else if albums.length === 0}
    <div class="empty-state">
      このフォルダーには、フォルダーも曲もありません。<br />
      上のパスや [↑ 上へ] で、ほかのフォルダーへ移動してください。
    </div>
  {:else}
    <div class="grid" bind:this={gridEl}>
      {#each albums as album, index}
        {@const art = artworkCache[album.first_track_path]}
        <div
          class="album-card"
          class:folder={album.is_folder}
          title={album.is_folder ? `${album.album_name} を開く` : undefined}
          role="button"
          data-nav-item
          tabindex={index === activeCard ? 0 : -1}
          onclick={() => onAlbumClick(album)}
          onfocus={() => (activeIndex = index)}
          onkeydown={(e) => onCardKeydown(e, album, index)}
        >
          <div class="art-container">
            {#if album.is_folder}
              <div class="art-placeholder">
                <span class="disc-icon">📁</span>
              </div>
            {:else if art}
              <img src={art} alt={album.album_name} class="art-img" />
            {:else}
              <div class="art-placeholder">
                <span class="disc-icon">💿</span>
              </div>
            {/if}
            {#if !album.is_folder}
              <span class="track-count">{album.track_count}曲</span>
            {/if}
          </div>

          <div class="album-info">
            <div class="album-title" title={album.album_name}>
              {album.album_name}
            </div>
            <div class="album-artist" title={album.artist_name}>
              {album.is_folder ? "フォルダー" : album.artist_name}
            </div>
          </div>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .artwork-grid-view {
    flex: 1;
    overflow-y: auto;
    padding: 16px;
    background: var(--nord0);
  }

  .empty-state {
    padding: 60px 20px;
    text-align: center;
    color: var(--ui-text-muted);
    line-height: 1.6;
  }

  .grid {
    display: grid;
    grid-template-columns: repeat(auto-fill, minmax(160px, 1fr));
    gap: 16px;
  }

  .album-card {
    background: var(--nord1);
    border: 1px solid var(--nord3);
    border-radius: 4px;
    overflow: hidden;
    cursor: pointer;
    transition: transform 0.1s ease, border-color 0.1s ease;
    display: flex;
    flex-direction: column;
    outline: none;
  }

  .album-card:hover {
    transform: translateY(-2px);
    border-color: var(--nord8);
  }

  .art-container {
    width: 100%;
    aspect-ratio: 1 / 1;
    background: var(--nord0);
    position: relative;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
  }

  .art-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .art-placeholder {
    display: flex;
    align-items: center;
    justify-content: center;
    width: 100%;
    height: 100%;
    background: var(--nord2);
  }

  .disc-icon {
    font-size: 48px;
    opacity: 0.6;
  }

  .track-count {
    position: absolute;
    bottom: 6px;
    right: 6px;
    background: color-mix(in srgb, var(--nord0) 85%, transparent);
    color: var(--nord4);
    font-size: 10px;
    padding: 2px 6px;
    border-radius: 3px;
    border: 1px solid var(--nord3);
  }

  .album-info {
    padding: 8px 10px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .album-title {
    font-size: 12px;
    font-weight: bold;
    color: var(--nord6);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .album-artist {
    font-size: 11px;
    color: var(--nord4);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }
</style>
