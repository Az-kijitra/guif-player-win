<script lang="ts">
  import { playerState } from "../playerState.svelte";

  let track = $derived(playerState.playlistModalTrack);
  let newName = $state("");
  let cardEl = $state<HTMLDivElement | null>(null);

  // Keyboard users start inside the dialog and return to the row that opened it
  $effect(() => {
    if (!cardEl) return;
    const opener = document.activeElement as HTMLElement | null;
    cardEl.querySelector<HTMLElement>(".playlist-option-btn, input")?.focus();
    return () => opener?.focus();
  });

  async function createAndAdd(e: SubmitEvent) {
    e.preventDefault();
    if (!newName.trim() || !track) return;
    await playerState.createPlaylistAndAdd(newName.trim(), track);
    newName = "";
  }
</script>

{#if track}
  <div
    class="modal-backdrop"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={() => (playerState.playlistModalTrack = null)}
    onkeydown={(e) => {
      if (e.key === "Escape") playerState.playlistModalTrack = null;
    }}
  >
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="modal-card"
      role="document"
      tabindex="-1"
      bind:this={cardEl}
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => {
        if (e.key !== "Escape") e.stopPropagation();
      }}
    >
      <div class="modal-header">
        <h3>プレイリストに追加</h3>
        <button
          class="close-btn"
          onclick={() => (playerState.playlistModalTrack = null)}
        >
          ×
        </button>
      </div>

      <div class="track-preview">
        <span class="preview-title">{track.title}</span>
        <span class="preview-artist">{track.artist}</span>
      </div>

      <!-- Add to existing -->
      <div class="existing-section">
        <span class="section-label">既存のプレイリスト:</span>
        <div class="playlist-options">
          {#if playerState.playlists.length === 0}
            <div class="no-playlists">プレイリストがありません</div>
          {:else}
            {#each playerState.playlists as p}
              <button
                class="playlist-option-btn"
                onclick={() => playerState.addTrackToPlaylist(p.id, track!)}
              >
                📜 {p.name} ({p.track_count}曲)
              </button>
            {/each}
          {/if}
        </div>
      </div>

      <!-- Create new -->
      <form class="create-new" onsubmit={createAndAdd}>
        <input
          type="text"
          placeholder="新しいプレイリストを作成して追加..."
          bind:value={newName}
          class="create-input"
        />
        <button type="submit" class="create-btn">作成＆追加</button>
      </form>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.65);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 120;
  }

  .modal-card {
    background: var(--nord1);
    border: 1px solid var(--nord3);
    border-radius: 6px;
    width: 380px;
    padding: 16px;
    display: flex;
    flex-direction: column;
    gap: 12px;
    box-shadow: 0 8px 24px rgba(0, 0, 0, 0.4);
  }

  .modal-header {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .modal-header h3 {
    margin: 0;
    font-size: 14px;
    color: var(--nord6);
  }

  .close-btn {
    font-size: 18px;
    color: var(--ui-text-muted);
  }

  .close-btn:hover {
    color: var(--nord11);
  }

  .track-preview {
    padding: 8px 10px;
    background: var(--nord0);
    border: 1px solid var(--nord3);
    border-radius: 4px;
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .preview-title {
    font-size: 12px;
    font-weight: bold;
    color: var(--nord8);
  }

  .preview-artist {
    font-size: 11px;
    color: var(--nord4);
  }

  .section-label {
    font-size: 11px;
    color: var(--nord4);
    margin-bottom: 4px;
    display: block;
  }

  .playlist-options {
    max-height: 140px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .playlist-option-btn {
    text-align: left;
    padding: 6px 10px;
    background: var(--nord2);
    border: 1px solid var(--nord3);
    border-radius: 3px;
    font-size: 12px;
    color: var(--nord6);
  }

  .playlist-option-btn:hover {
    background: var(--nord8);
    color: var(--nord0);
  }

  .no-playlists {
    font-size: 11px;
    color: var(--ui-text-muted);
    padding: 6px 0;
  }

  .create-new {
    display: flex;
    gap: 6px;
    margin-top: 4px;
  }

  .create-input {
    flex: 1;
    font-size: 11px;
  }

  .create-btn {
    padding: 4px 8px;
    background: var(--nord8);
    color: var(--nord0);
    font-weight: bold;
    font-size: 11px;
    border-radius: 2px;
    white-space: nowrap;
  }
</style>
