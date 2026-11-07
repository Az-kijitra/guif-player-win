<script lang="ts">
  import { PLAYBACK_SPEEDS, formatSpeed } from "../speed";
  import { formatTime, playerState } from "../playerState.svelte";

  let track = $derived(playerState.playback.current_track);
  let isPlaying = $derived(playerState.playback.is_playing);
  let isSeeking = $state(false);
  let seekVal = $state(0);

  let currentPos = $derived(
    isSeeking ? seekVal : playerState.playback.position_ms
  );
  let duration = $derived(playerState.playback.duration_ms);

  function onSeekInput(e: Event) {
    isSeeking = true;
    seekVal = Number((e.target as HTMLInputElement).value);
  }

  function onSeekChange(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    playerState.seek(val);
    isSeeking = false;
  }
  let hasBookmark = $derived(
    playerState.bookmarks.some(
      (b) => b.file_path === playerState.playback.current_track?.file_path
    )
  );
</script>

{#if playerState.showDetailDialog}
  <div
    class="modal-backdrop"
    role="dialog"
    aria-modal="true"
    tabindex="-1"
    onclick={() => (playerState.showDetailDialog = false)}
    onkeydown={(e) => {
      if (e.key === "Escape") {
        // Handled here: the window's Esc (such as closing an album) must not run too
        e.stopPropagation();
        playerState.showDetailDialog = false;
      }
    }}
  >
    <!-- Modal Card (stop propagation on click) -->
    <!-- svelte-ignore a11y_no_noninteractive_element_interactions -->
    <div
      class="modal-card"
      role="document"
      tabindex="-1"
      onclick={(e) => e.stopPropagation()}
      onkeydown={(e) => e.stopPropagation()}
    >
      <!-- Close Button -->
      <button
        class="close-btn"
        onclick={() => (playerState.showDetailDialog = false)}
        title="閉じる (Esc)"
      >
        ×
      </button>

      <div class="content-layout">
        <!-- Left: Large Art & Track Info -->
        <div class="left-pane">
          <div class="large-art">
            {#if playerState.currentArtwork}
              <img src={playerState.currentArtwork} alt="Cover" class="large-img" />
            {:else}
              <div class="placeholder-disc">💿</div>
            {/if}
          </div>

          <div class="track-details">
            <h2 class="detail-title">{track?.title ?? "再生していません"}</h2>
            <div class="detail-artist">{track?.artist ?? "Unknown Artist"}</div>
            <div class="detail-album">{track?.album ?? "Unknown Album"}</div>
          </div>

          <!-- Timeline -->
          <div class="modal-timeline">
            <span class="time-label">{formatTime(currentPos)}</span>
            <input
              type="range"
              min="0"
              max={duration || 100}
              value={currentPos}
              oninput={onSeekInput}
              onchange={onSeekChange}
              class="large-progress"
            />
            <span class="time-label">{formatTime(duration)}</span>
          </div>

          <!-- Controls -->
          <div class="modal-controls">
            <button
              class="modal-ctrl"
              class:active={playerState.playback.shuffle_mode}
              onclick={() => playerState.toggleShuffle()}
            >
              🔀
            </button>
            <button
              class="modal-ctrl"
              onclick={() => playerState.playPrevious()}
            >
              ⏮
            </button>
            <button
              class="modal-ctrl modal-play"
              onclick={() => playerState.togglePlayPause()}
            >
              {isPlaying ? "⏸" : "▶"}
            </button>
            <button
              class="modal-ctrl"
              onclick={() => playerState.playNext()}
            >
              ⏭
            </button>
            <button
              class="modal-ctrl"
              class:active={playerState.playback.repeat_mode > 0}
              onclick={() => playerState.toggleRepeat()}
            >
              {playerState.playback.repeat_mode === 1 ? "🔂" : "🔁"}
            </button>
          </div>
          <div class="extra-controls">
          <label class="speed-select" title="再生速度 ([ で遅く、] で速く)">
            速度
            <select
              value={playerState.playback.speed}
              onchange={(e) => playerState.setPlaybackSpeed(Number((e.target as HTMLSelectElement).value))}
            >
              {#each PLAYBACK_SPEEDS as speed}
                <option value={speed}>{formatSpeed(speed)}</option>
              {/each}
            </select>
          </label>
          <button
            class="bookmark-btn"
            class:has-bookmark={hasBookmark}
            title={hasBookmark ? "しおりを今の位置に挟み直す (B)" : "今の位置にしおりを挟む (B)"}
            onclick={() => playerState.addBookmark()}
          >
            {hasBookmark ? "★" : "☆"} しおり
          </button>
          </div>
        </div>

        <!-- Right: Playback Queue -->
        <div class="right-pane">
          <div class="queue-header">
            再生キュー ({playerState.queue.length} 曲)
          </div>

          <div class="queue-list">
            {#each playerState.queue as qTrack, idx}
              {@const isCur = qTrack.file_path === track?.file_path}
              <div
                class="queue-item"
                class:is-active={isCur}
                role="button"
                tabindex="0"
                onclick={() =>
                  playerState.playTrackInList(
                    qTrack,
                    playerState.queue
                  )}
                onkeydown={(e) => {
                  if (e.key === "Enter")
                    playerState.playTrackInList(
                      qTrack,
                      playerState.queue
                    );
                }}
              >
                <span class="q-num">{idx + 1}</span>
                <div class="q-info">
                  <span class="q-title">{qTrack.title}</span>
                  <span class="q-artist">{qTrack.artist}</span>
                </div>
                <span class="q-dur">{formatTime(qTrack.duration_ms)}</span>
              </div>
            {/each}
          </div>
        </div>
      </div>
    </div>
  </div>
{/if}

<style>
  .extra-controls {
    display: flex;
    align-items: center;
    justify-content: center;
    gap: 12px;
    margin-top: 12px;
  }

  .speed-select {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 12px;
    color: var(--nord4);
  }

  .speed-select select {
    padding: 3px 6px;
    color: var(--nord4);
    background: var(--nord1);
    border: 1px solid var(--nord3);
    border-radius: 3px;
    font-family: inherit;
  }

  .bookmark-btn {
    padding: 4px 14px;
    border: 1px solid var(--nord3);
    border-radius: 3px;
    color: var(--nord4);
    cursor: pointer;
  }

  .bookmark-btn.has-bookmark {
    color: var(--nord13);
    border-color: var(--nord13);
  }

  .modal-backdrop {
    position: fixed;
    top: 0;
    left: 0;
    width: 100vw;
    height: 100vh;
    background: rgba(0, 0, 0, 0.75);
    display: flex;
    align-items: center;
    justify-content: center;
    z-index: 100;
    backdrop-filter: blur(4px);
  }

  .modal-card {
    background: var(--nord1);
    border: 1px solid var(--nord3);
    border-radius: 8px;
    width: 820px;
    max-width: 90vw;
    height: 520px;
    max-height: 90vh;
    display: flex;
    flex-direction: column;
    position: relative;
    box-shadow: 0 10px 30px rgba(0, 0, 0, 0.5);
    overflow: hidden;
  }

  .close-btn {
    position: absolute;
    top: 10px;
    right: 14px;
    font-size: 24px;
    color: var(--nord4);
    z-index: 10;
  }

  .close-btn:hover {
    color: var(--nord11);
  }

  .content-layout {
    display: flex;
    height: 100%;
    overflow: hidden;
  }

  .left-pane {
    width: 420px;
    padding: 24px;
    display: flex;
    flex-direction: column;
    align-items: center;
    border-right: 1px solid var(--nord3);
    background: var(--nord0);
  }

  .large-art {
    width: 200px;
    height: 200px;
    background: var(--nord1);
    border-radius: 6px;
    overflow: hidden;
    border: 1px solid var(--nord3);
    display: flex;
    align-items: center;
    justify-content: center;
    box-shadow: 0 6px 16px rgba(0, 0, 0, 0.4);
  }

  .large-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .placeholder-disc {
    font-size: 72px;
    opacity: 0.5;
  }

  .track-details {
    margin-top: 16px;
    text-align: center;
    width: 100%;
  }

  .detail-title {
    margin: 0;
    font-size: 16px;
    color: var(--nord6);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .detail-artist {
    font-size: 13px;
    color: var(--nord8);
    margin-top: 4px;
  }

  .detail-album {
    font-size: 11px;
    color: var(--ui-text-muted);
    margin-top: 2px;
  }

  .modal-timeline {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
    margin-top: 20px;
  }

  .time-label {
    font-size: 11px;
    color: var(--ui-text-muted);
    width: 44px;
    text-align: center;
    font-variant-numeric: tabular-nums;
  }

  .large-progress {
    flex: 1;
    height: 6px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--nord2);
    border-radius: 3px;
    outline: none;
  }

  .large-progress::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 14px;
    height: 14px;
    border-radius: 50%;
    background: var(--nord8);
    cursor: pointer;
  }

  .modal-controls {
    display: flex;
    align-items: center;
    gap: 16px;
    margin-top: 20px;
  }

  .modal-ctrl {
    font-size: 18px;
    padding: 6px;
    color: var(--nord4);
    border-radius: 4px;
  }

  .modal-ctrl:hover {
    color: var(--nord6);
  }

  .modal-ctrl.active {
    color: var(--nord8);
    background: var(--nord2);
  }

  .modal-play {
    font-size: 26px;
    width: 46px;
    height: 46px;
    border-radius: 50%;
    background: var(--nord8);
    color: var(--nord0);
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .modal-play:hover {
    background: var(--nord7);
  }

  .right-pane {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .queue-header {
    padding: 12px 16px;
    font-size: 12px;
    font-weight: bold;
    color: var(--nord4);
    border-bottom: 1px solid var(--nord3);
    background: var(--nord1);
  }

  .queue-list {
    flex: 1;
    overflow-y: auto;
  }

  .queue-item {
    display: flex;
    align-items: center;
    padding: 8px 16px;
    border-bottom: 1px solid color-mix(in srgb, var(--nord3) 25%, transparent);
    cursor: pointer;
    font-size: 12px;
    gap: 8px;
  }

  .queue-item:hover {
    background: var(--nord2);
  }

  .queue-item.is-active {
    background: color-mix(in srgb, var(--nord8) 15%, transparent);
    color: var(--nord8);
    font-weight: bold;
  }

  .q-num {
    width: 24px;
    color: var(--ui-text-muted);
  }

  .q-info {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .q-title {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .q-artist {
    font-size: 10px;
    color: var(--ui-text-muted);
  }

  .q-dur {
    color: var(--ui-text-muted);
    font-size: 11px;
  }
</style>
