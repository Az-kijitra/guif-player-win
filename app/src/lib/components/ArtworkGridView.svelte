<script lang="ts">
  import { onMount } from "svelte";
  import { api } from "../api";
  import { playerState } from "../playerState.svelte";
  import type { AlbumSummary } from "../types";

  let albums = $derived(playerState.albums);
  let artworkCache = $state<Record<string, string | null>>({});

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

  async function onAlbumClick(album: AlbumSummary) {
    // Navigate into the album's folder
    await playerState.navigateTo(album.folder_path);
    playerState.setViewMode("FOLDER");
  }
</script>

<div class="artwork-grid-view">
  {#if albums.length === 0}
    <div class="empty-state">
      このディレクトリ配下にはアルバムが見つかりませんでした。<br />
      上の [📁 フォルダ] から音楽フォルダを選択してください。
    </div>
  {:else}
    <div class="grid">
      {#each albums as album}
        {@const art = artworkCache[album.first_track_path]}
        <div
          class="album-card"
          role="button"
          tabindex="0"
          onclick={() => onAlbumClick(album)}
          onkeydown={(e) => {
            if (e.key === "Enter") onAlbumClick(album);
          }}
        >
          <div class="art-container">
            {#if art}
              <img src={art} alt={album.album_name} class="art-img" />
            {:else}
              <div class="art-placeholder">
                <span class="disc-icon">💿</span>
              </div>
            {/if}
            <span class="track-count">{album.track_count}曲</span>
          </div>

          <div class="album-info">
            <div class="album-title" title={album.album_name}>
              {album.album_name}
            </div>
            <div class="album-artist" title={album.artist_name}>
              {album.artist_name}
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
    color: var(--nord3);
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
