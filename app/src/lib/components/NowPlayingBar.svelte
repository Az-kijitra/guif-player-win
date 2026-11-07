<script lang="ts">
  import { speedLabel } from "../speed";
  import { formatTime, playerState } from "../playerState.svelte";

  let isSeeking = $state(false);
  let seekVal = $state(0);

  let currentPos = $derived(
    isSeeking ? seekVal : playerState.playback.position_ms
  );
  let duration = $derived(playerState.playback.duration_ms);
  let track = $derived(playerState.playback.current_track);
  let isPlaying = $derived(playerState.playback.is_playing);

  function onSeekInput(e: Event) {
    isSeeking = true;
    seekVal = Number((e.target as HTMLInputElement).value);
  }

  function onSeekChange(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    playerState.seek(val);
    isSeeking = false;
  }

  function onVolumeChange(e: Event) {
    const val = Number((e.target as HTMLInputElement).value);
    playerState.setVolume(val);
  }
</script>

<footer class="now-playing-bar">
  <!-- Left: Track Info & Artwork -->
  <div
    class="track-info"
    role="button"
    tabindex="0"
    onclick={() => (playerState.showDetailDialog = true)}
    onkeydown={(e) => {
      if (e.key === "Enter") playerState.showDetailDialog = true;
    }}
    title="詳細プレイヤーを開く"
  >
    <div class="art-thumb">
      {#if playerState.currentArtwork}
        <img src={playerState.currentArtwork} alt="Cover" class="thumb-img" />
      {:else}
        <div class="thumb-placeholder">🎵</div>
      {/if}
    </div>
    <div class="meta">
      <span class="title">
        {track?.title ?? "再生していません"}
        {#if speedLabel(playerState.playback.speed)}
          <span class="speed-badge" title="再生速度">{speedLabel(playerState.playback.speed)}</span>
        {/if}
      </span>
      <span class="artist">{track?.artist ?? "guif-player"}</span>
    </div>
  </div>

  <!-- Center: Controls & Timeline -->
  <div class="center-controls">
    <div class="buttons">
      <!-- Shuffle -->
      <button
        class="ctrl-btn"
        class:active={playerState.playback.shuffle_mode}
        onclick={() => playerState.toggleShuffle()}
        title="シャッフル"
      >
        🔀
      </button>

      <!-- Previous -->
      <button
        class="ctrl-btn"
        onclick={() => playerState.playPrevious()}
        title="前の曲 (Ctrl+Left)"
      >
        ⏮
      </button>

      <!-- Play / Pause -->
      <button
        class="ctrl-btn play-btn"
        onclick={() => playerState.togglePlayPause()}
        title="再生 / 一時停止 (Space)"
      >
        {isPlaying ? "⏸" : "▶"}
      </button>

      <!-- Next -->
      <button
        class="ctrl-btn"
        onclick={() => playerState.playNext()}
        title="次の曲 (Ctrl+Right)"
      >
        ⏭
      </button>

      <!-- Repeat -->
      <button
        class="ctrl-btn repeat-btn"
        class:active={playerState.playback.repeat_mode > 0}
        onclick={() => playerState.toggleRepeat()}
        title="リピート切替"
      >
        {#if playerState.playback.repeat_mode === 1}
          🔂
        {:else}
          🔁
        {/if}
      </button>
    </div>

    <!-- Timeline Slider -->
    <div class="timeline">
      <span class="time-label">{formatTime(currentPos)}</span>
      <input
        type="range"
        min="0"
        max={duration || 100}
        value={currentPos}
        oninput={onSeekInput}
        onchange={onSeekChange}
        class="progress-bar"
      />
      <span class="time-label">{formatTime(duration)}</span>
    </div>
  </div>

  <!-- Right: Volume & Expand -->
  <div class="right-controls">
    <span class="vol-icon">🔊</span>
    <input
      type="range"
      min="0"
      max="1"
      step="0.01"
      value={playerState.playback.volume}
      oninput={onVolumeChange}
      class="vol-slider"
      title="音量"
    />
    <button
      class="expand-btn"
      onclick={() => (playerState.showDetailDialog = true)}
      title="拡大表示"
    >
      ⛶
    </button>
  </div>
</footer>

<style>
  .now-playing-bar {
    height: 72px;
    background: var(--nord1);
    border-top: 1px solid var(--nord3);
    display: flex;
    align-items: center;
    justify-content: space-between;
    padding: 0 16px;
    gap: 16px;
    z-index: 20;
  }

  .track-info {
    width: 240px;
    display: flex;
    align-items: center;
    gap: 12px;
    cursor: pointer;
    overflow: hidden;
  }

  .speed-badge {
    margin-left: 6px;
    padding: 0 4px;
    font-size: 10px;
    font-weight: normal;
    color: var(--nord0);
    background: var(--nord8);
    border-radius: 2px;
  }

  .track-info:hover .title {
    color: var(--nord8);
  }

  .art-thumb {
    width: 48px;
    height: 48px;
    background: var(--nord0);
    border-radius: 4px;
    overflow: hidden;
    display: flex;
    align-items: center;
    justify-content: center;
    flex-shrink: 0;
    border: 1px solid var(--nord3);
  }

  .thumb-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .thumb-placeholder {
    font-size: 20px;
    opacity: 0.6;
  }

  .meta {
    display: flex;
    flex-direction: column;
    overflow: hidden;
    gap: 2px;
  }

  .title {
    font-size: 13px;
    font-weight: bold;
    color: var(--nord6);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .artist {
    font-size: 11px;
    color: var(--nord4);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .center-controls {
    flex: 1;
    max-width: 540px;
    display: flex;
    flex-direction: column;
    align-items: center;
    gap: 4px;
  }

  .buttons {
    display: flex;
    align-items: center;
    gap: 16px;
  }

  .ctrl-btn {
    font-size: 16px;
    padding: 4px 6px;
    color: var(--nord4);
    border-radius: 4px;
  }

  .ctrl-btn:hover {
    color: var(--nord6);
  }

  .ctrl-btn.active {
    color: var(--nord8);
    background: var(--nord2);
  }

  .play-btn {
    font-size: 22px;
    width: 36px;
    height: 36px;
    background: var(--nord8);
    color: var(--nord0);
    border-radius: 50%;
    display: flex;
    align-items: center;
    justify-content: center;
    padding: 0;
  }

  .play-btn:hover {
    background: var(--nord7);
    color: var(--nord0);
  }

  .timeline {
    width: 100%;
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .time-label {
    font-size: 11px;
    font-variant-numeric: tabular-nums;
    color: var(--ui-text-muted);
    width: 40px;
    text-align: center;
  }

  .progress-bar {
    flex: 1;
    height: 4px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--nord2);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }

  .progress-bar::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--nord8);
    cursor: pointer;
    transition: transform 0.1s ease;
  }

  .progress-bar::-webkit-slider-thumb:hover {
    transform: scale(1.3);
  }

  .right-controls {
    width: 180px;
    display: flex;
    align-items: center;
    gap: 8px;
    justify-content: flex-end;
  }

  .vol-icon {
    font-size: 14px;
    color: var(--nord4);
  }

  .vol-slider {
    width: 80px;
    height: 4px;
    -webkit-appearance: none;
    appearance: none;
    background: var(--nord2);
    border-radius: 2px;
    outline: none;
    cursor: pointer;
  }

  .vol-slider::-webkit-slider-thumb {
    -webkit-appearance: none;
    appearance: none;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    background: var(--nord8);
    cursor: pointer;
  }

  .expand-btn {
    padding: 4px 8px;
    color: var(--nord4);
    font-size: 16px;
    border-radius: 4px;
  }

  .expand-btn:hover {
    background: var(--nord2);
    color: var(--nord6);
  }
</style>
