<script lang="ts">
  import { api } from "../api";
  import { formatTime, playerState } from "../playerState.svelte";
  import { ARTWORK_IMAGE_TYPES, describeTagUpdate } from "../tagUpdateMessage";
  import type { AudioTrack, TagUpdateRequest } from "../types";

  let tracks = $derived(playerState.selectedTracks);
  let count = $derived(tracks.length);
  let isBatch = $derived(count > 1);

  // Form values
  let title = $state("");
  let artist = $state("");
  let album = $state("");
  let trackNumberStr = $state("");

  // Dirty flags (whether user modified each field)
  let titleDirty = $state(false);
  let artistDirty = $state(false);
  let albumDirty = $state(false);
  let trackNumDirty = $state(false);

  // Batch / Quick options
  let filenameToTitle = $state(false);
  let autoNumberTracks = $state(false);
  let clearTrackNumber = $state(false);

  // Artwork state
  let previewArtwork = $state<string | null>(null);
  let newArtworkDataUrl = $state<string | null>(null);
  let clearArtwork = $state(false);
  let isDragOver = $state(false);
  let fileInputEl = $state<HTMLInputElement | null>(null);

  // Save status feedback
  let isSaving = $state(false);
  let statusMessage = $state<{ text: string; type: "success" | "error" } | null>(null);

  // Check if selected tracks have mixed values per property
  let mixedTitle = $derived(
    isBatch && new Set(tracks.map((t) => t.title)).size > 1
  );
  let mixedArtist = $derived(
    isBatch && new Set(tracks.map((t) => t.artist)).size > 1
  );
  let mixedAlbum = $derived(
    isBatch && new Set(tracks.map((t) => t.album)).size > 1
  );
  let mixedTrackNum = $derived(
    isBatch && new Set(tracks.map((t) => t.track_number ?? null)).size > 1
  );

  let hasChanges = $derived(
    titleDirty ||
      artistDirty ||
      albumDirty ||
      trackNumDirty ||
      filenameToTitle ||
      autoNumberTracks ||
      clearTrackNumber ||
      newArtworkDataUrl !== null ||
      clearArtwork
  );

  // Signature of current selection to reset form when selection changes
  let selectionSignature = $derived(
    tracks
      .map(
        (t) =>
          `${t.file_path}|${t.title}|${t.artist}|${t.album}|${t.track_number ?? ""}|${playerState.artworkVersion}`
      )
      .join("::")
  );

  $effect(() => {
    // Re-initialize form whenever selection or track metadata updates
    const _sig = selectionSignature;
    resetFormFromSelection();
  });

  async function resetFormFromSelection() {
    titleDirty = false;
    artistDirty = false;
    albumDirty = false;
    trackNumDirty = false;
    filenameToTitle = false;
    autoNumberTracks = false;
    clearTrackNumber = false;
    newArtworkDataUrl = null;
    clearArtwork = false;

    if (tracks.length === 0) {
      title = "";
      artist = "";
      album = "";
      trackNumberStr = "";
      previewArtwork = null;
      return;
    }

    const first = tracks[0];
    const allSameTitle = tracks.every((t) => t.title === first.title);
    const allSameArtist = tracks.every((t) => t.artist === first.artist);
    const allSameAlbum = tracks.every((t) => t.album === first.album);
    const allSameTrackNum = tracks.every(
      (t) => t.track_number === first.track_number
    );

    title = allSameTitle ? first.title : "";
    artist = allSameArtist ? first.artist : "";
    album = allSameAlbum ? first.album : "";
    trackNumberStr =
      allSameTrackNum && first.track_number != null
        ? String(first.track_number)
        : "";

    // Load artwork from the first track in selection
    try {
      previewArtwork = await api.getArtwork(first.file_path);
    } catch {
      previewArtwork = null;
    }
  }

  function handleImageFile(file: File) {
    if (!ARTWORK_IMAGE_TYPES.includes(file.type)) {
      statusMessage = {
        text: "対応していない画像形式です。JPEG、PNG、GIF を選んでください",
        type: "error",
      };
      return;
    }
    const reader = new FileReader();
    reader.onload = () => {
      if (typeof reader.result === "string") {
        newArtworkDataUrl = reader.result;
        previewArtwork = reader.result;
        clearArtwork = false;
      }
    };
    reader.readAsDataURL(file);
  }

  function onFileInputChange(e: Event) {
    const input = e.target as HTMLInputElement;
    if (input.files && input.files[0]) {
      handleImageFile(input.files[0]);
    }
  }

  function onDropImage(e: DragEvent) {
    e.preventDefault();
    isDragOver = false;
    if (e.dataTransfer?.files && e.dataTransfer.files[0]) {
      handleImageFile(e.dataTransfer.files[0]);
    }
  }

  export async function saveChanges() {
    if (tracks.length === 0 || !hasChanges || isSaving) return;
    isSaving = true;
    statusMessage = null;

    const parsedTrackNum =
      trackNumDirty && trackNumberStr.trim() !== ""
        ? parseInt(trackNumberStr.trim(), 10)
        : null;

    const shouldClearTrackNum =
      clearTrackNumber || (trackNumDirty && trackNumberStr.trim() === "");

    const req: TagUpdateRequest = {
      file_paths: tracks.map((t) => t.file_path),
      title: titleDirty ? title.trim() : null,
      artist: artistDirty ? artist.trim() : null,
      album: albumDirty ? album.trim() : null,
      track_number:
        parsedTrackNum !== null && !isNaN(parsedTrackNum) ? parsedTrackNum : null,
      clear_track_number: shouldClearTrackNum,
      auto_number_tracks: autoNumberTracks,
      filename_to_title: filenameToTitle,
      artwork_data_url: newArtworkDataUrl,
      clear_artwork: clearArtwork,
    };

    try {
      const result = await playerState.updateTracksMetadata(req);
      statusMessage = describeTagUpdate(result);
      setTimeout(() => {
        if (statusMessage?.type === "success") {
          statusMessage = null;
        }
      }, 3000);
    } catch (err) {
      statusMessage = {
        text: `エラー: ${String(err)}`,
        type: "error",
      };
    } finally {
      isSaving = false;
    }
  }
</script>

{#if playerState.showInspector}
  <aside class="inspector-panel">
    <!-- Header -->
    <div class="inspector-header">
      <div class="header-left">
        <span class="header-icon">🏷</span>
        <span class="header-title">プロパティ</span>
        {#if count > 0}
          <span class="selection-badge" class:batch={isBatch}>
            {isBatch ? `${count}件 一括編集` : "1件選択中"}
          </span>
        {/if}
      </div>

      <div class="header-actions">
        {#if count > 0}
          <button
            class="clear-sel-btn"
            onclick={() => playerState.clearSelection()}
            title="選択をすべて解除"
          >
            解除
          </button>
        {/if}
        <button
          class="close-panel-btn"
          onclick={() => (playerState.showInspector = false)}
          title="パネルを閉じる (Ctrl+I)"
        >
          ×
        </button>
      </div>
    </div>

    {#if count === 0}
      <!-- Empty Selection State -->
      <div class="empty-inspector">
        <div class="empty-icon">🏷</div>
        <p class="empty-text">
          曲を選択すると、ここでタイトル・アーティスト・アルバム・トラック番号・ジャケット画像を編集できます。
        </p>
        <div class="shortcut-hints">
          <div><span>クリック</span> 単一選択＆再生</div>
          <div><span>チェック / Ctrl+Click</span> 複数選択</div>
          <div><span>Shift+Click</span> 範囲選択</div>
          <div><span>Ctrl + A</span> すべて選択</div>
          <div><span>Ctrl + S</span> 変更を保存</div>
        </div>

        {#if playerState.playback.current_track}
          <button
            class="select-playing-btn"
            onclick={() =>
              playerState.selectSingleTrack(playerState.playback.current_track!)}
          >
            🎵 現在再生中の曲を編集
          </button>
        {/if}
      </div>
    {:else}
      <!-- Inspector Body -->
      <form
        class="inspector-form"
        onsubmit={(e) => {
          e.preventDefault();
          saveChanges();
        }}
      >
        <!-- Artwork Section -->
        <div class="artwork-section">
          <div
            class="artwork-box"
            class:drag-over={isDragOver}
            role="button"
            tabindex="0"
            onclick={() => fileInputEl?.click()}
            onkeydown={(e) => {
              if (e.key === "Enter") fileInputEl?.click();
            }}
            ondragover={(e) => {
              e.preventDefault();
              isDragOver = true;
            }}
            ondragleave={() => (isDragOver = false)}
            ondrop={onDropImage}
            title="クリックまたは画像をドロップしてジャケット変更"
          >
            {#if clearArtwork}
              <div class="art-empty">画像削除予定</div>
            {:else if previewArtwork}
              <img src={previewArtwork} alt="Cover" class="art-preview-img" />
            {:else}
              <div class="art-empty">
                <span>🖼</span>
                <small>画像なし</small>
              </div>
            {/if}
          </div>

          <input
            type="file"
            accept={ARTWORK_IMAGE_TYPES.join(",")}
            bind:this={fileInputEl}
            onchange={onFileInputChange}
            style="display: none;"
          />

          <div class="artwork-controls">
            <div class="artwork-status">
              {#if newArtworkDataUrl}
                <span class="dirty-tag">新しい画像を設定</span>
              {:else if clearArtwork}
                <span class="dirty-tag warn">埋め込み画像を削除</span>
              {:else if isBatch}
                <span class="muted-note">代表1件目の画像を表示中</span>
              {:else}
                <span class="muted-note">ジャケット画像</span>
              {/if}
            </div>
            <div class="art-btns">
              <button
                type="button"
                class="mini-btn"
                onclick={() => fileInputEl?.click()}
              >
                画像を選択...
              </button>
              <button
                type="button"
                class="mini-btn danger"
                onclick={() => {
                  newArtworkDataUrl = null;
                  previewArtwork = null;
                  clearArtwork = true;
                }}
              >
                削除
              </button>
            </div>
          </div>
        </div>

        <!-- Fields -->
        <div class="field-group">
          <div class="field-label-row">
            <label for="tag-title">タイトル</label>
            {#if titleDirty || filenameToTitle}
              <span class="dirty-badge">変更あり</span>
            {/if}
          </div>
          <input
            id="tag-title"
            type="text"
            bind:value={title}
            disabled={filenameToTitle}
            placeholder={mixedTitle
              ? "<複数の値 - 変更しない>"
              : "タイトルを入力..."}
            oninput={() => (titleDirty = true)}
          />
          <label class="checkbox-option">
            <input type="checkbox" bind:checked={filenameToTitle} />
            <span>ファイル名（拡張子なし）をタイトルに設定</span>
          </label>
        </div>

        <div class="field-group">
          <div class="field-label-row">
            <label for="tag-artist">アーティスト</label>
            {#if artistDirty}
              <span class="dirty-badge">変更あり</span>
            {/if}
          </div>
          <input
            id="tag-artist"
            type="text"
            bind:value={artist}
            placeholder={mixedArtist
              ? "<複数の値 - 変更しない>"
              : "アーティストを入力..."}
            oninput={() => (artistDirty = true)}
          />
        </div>

        <div class="field-group">
          <div class="field-label-row">
            <label for="tag-album">アルバム</label>
            {#if albumDirty}
              <span class="dirty-badge">変更あり</span>
            {/if}
          </div>
          <input
            id="tag-album"
            type="text"
            bind:value={album}
            placeholder={mixedAlbum
              ? "<複数の値 - 変更しない>"
              : "アルバム名を入力..."}
            oninput={() => (albumDirty = true)}
          />
          {#if tracks.length > 0}
            {@const folderName =
              tracks[0].file_path.split(/[/\\]/).slice(-2, -1)[0] ?? ""}
            {#if folderName}
              <button
                type="button"
                class="inline-helper-btn"
                onclick={() => {
                  album = folderName;
                  albumDirty = true;
                }}
              >
                📁 フォルダ名「{folderName}」をセット
              </button>
            {/if}
          {/if}
        </div>

        <div class="field-group">
          <div class="field-label-row">
            <label for="tag-track">トラック番号</label>
            {#if trackNumDirty || autoNumberTracks || clearTrackNumber}
              <span class="dirty-badge">変更あり</span>
            {/if}
          </div>
          <input
            id="tag-track"
            type="number"
            min="1"
            bind:value={trackNumberStr}
            disabled={autoNumberTracks || clearTrackNumber}
            placeholder={mixedTrackNum ? "<複数の値 - 変更しない>" : "例: 1"}
            oninput={() => {
              trackNumDirty = true;
              clearTrackNumber = false;
            }}
          />
          <div class="track-num-options">
            <label class="checkbox-option">
              <input
                type="checkbox"
                bind:checked={autoNumberTracks}
                onchange={() => {
                  if (autoNumberTracks) clearTrackNumber = false;
                }}
              />
              <span>選択順に連番を振る (1 〜 {count})</span>
            </label>
            <label class="checkbox-option">
              <input
                type="checkbox"
                bind:checked={clearTrackNumber}
                onchange={() => {
                  if (clearTrackNumber) autoNumberTracks = false;
                }}
              />
              <span>トラック番号を消去</span>
            </label>
          </div>
        </div>

        <!-- Status Message -->
        {#if statusMessage}
          <div
            class="status-banner"
            class:error={statusMessage.type === "error"}
          >
            {statusMessage.text}
          </div>
        {/if}

        <!-- Save & Reset Actions -->
        <div class="form-actions">
          <button
            type="submit"
            class="save-btn"
            disabled={!hasChanges || isSaving}
          >
            {isSaving
              ? "保存中..."
              : isBatch
                ? `💾 ${count}件を一括保存`
                : "💾 保存 (Ctrl+S)"}
          </button>
          <button
            type="button"
            class="reset-btn"
            disabled={!hasChanges || isSaving}
            onclick={() => resetFormFromSelection()}
          >
            リセット
          </button>
        </div>

        <!-- File info / Multi-selection order list -->
        {#if !isBatch && tracks[0]}
          <div class="file-meta-box">
            <div class="meta-row">
              <span class="meta-key">ファイル名:</span>
              <span class="meta-val" title={tracks[0].file_name}>
                {tracks[0].file_name}
              </span>
            </div>
            <div class="meta-row">
              <span class="meta-key">再生時間:</span>
              <span class="meta-val">{formatTime(tracks[0].duration_ms)}</span>
            </div>
            <div class="meta-row">
              <span class="meta-key">パス:</span>
              <span class="meta-val path" title={tracks[0].file_path}>
                {tracks[0].file_path}
              </span>
            </div>
          </div>
        {:else if isBatch}
          <div class="selected-list-section">
            <div class="selected-list-header">
              <span>選択ファイル一覧（連番順）</span>
              <span class="sub">{count}件</span>
            </div>
            <div class="selected-list">
              {#each tracks as t, idx}
                <div class="selected-row">
                  <span class="sel-idx">{idx + 1}.</span>
                  <span class="sel-name" title={t.file_name}>{t.title}</span>
                  <div class="sel-order-btns">
                    <button
                      type="button"
                      class="ord-btn"
                      disabled={idx === 0}
                      onclick={() => playerState.moveSelectedTrack(idx, -1)}
                      title="上へ移動"
                    >
                      ↑
                    </button>
                    <button
                      type="button"
                      class="ord-btn"
                      disabled={idx === count - 1}
                      onclick={() => playerState.moveSelectedTrack(idx, 1)}
                      title="下へ移動"
                    >
                      ↓
                    </button>
                    <button
                      type="button"
                      class="ord-btn remove"
                      onclick={() => playerState.toggleTrackSelection(t)}
                      title="選択から外す"
                    >
                      ×
                    </button>
                  </div>
                </div>
              {/each}
            </div>
          </div>
        {/if}
      </form>
    {/if}
  </aside>
{/if}

<style>
  .inspector-panel {
    width: 310px;
    flex-shrink: 0;
    background: var(--nord1);
    border-left: 1px solid var(--nord3);
    display: flex;
    flex-direction: column;
    height: 100%;
    overflow: hidden;
  }

  .inspector-header {
    padding: 8px 12px;
    background: var(--nord1);
    border-bottom: 1px solid var(--nord3);
    display: flex;
    align-items: center;
    justify-content: space-between;
  }

  .header-left {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .header-title {
    font-weight: bold;
    font-size: 12px;
    color: var(--nord6);
  }

  .selection-badge {
    font-size: 10px;
    padding: 1px 6px;
    background: var(--nord2);
    color: var(--nord8);
    border: 1px solid var(--nord3);
    border-radius: 3px;
  }

  .selection-badge.batch {
    background: color-mix(in srgb, var(--nord8) 20%, transparent);
    border-color: var(--nord8);
    font-weight: bold;
  }

  .header-actions {
    display: flex;
    align-items: center;
    gap: 6px;
  }

  .clear-sel-btn {
    font-size: 11px;
    padding: 2px 6px;
    background: var(--nord2);
    color: var(--nord4);
    border: 1px solid var(--nord3);
    border-radius: 2px;
  }

  .clear-sel-btn:hover {
    background: var(--nord3);
    color: var(--nord6);
  }

  .close-panel-btn {
    font-size: 16px;
    color: var(--nord4);
    padding: 0 4px;
  }

  .close-panel-btn:hover {
    color: var(--nord11);
  }

  .empty-inspector {
    padding: 24px 16px;
    display: flex;
    flex-direction: column;
    align-items: center;
    text-align: center;
    gap: 14px;
    color: var(--nord4);
  }

  .empty-icon {
    font-size: 32px;
    opacity: 0.5;
  }

  .empty-text {
    font-size: 12px;
    line-height: 1.6;
    color: var(--nord4);
    margin: 0;
  }

  .shortcut-hints {
    width: 100%;
    background: var(--nord0);
    border: 1px solid var(--nord3);
    border-radius: 4px;
    padding: 10px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
    font-size: 11px;
    text-align: left;
  }

  .shortcut-hints div {
    display: flex;
    justify-content: space-between;
    color: var(--nord4);
  }

  .shortcut-hints span {
    color: var(--nord8);
    font-weight: bold;
  }

  .select-playing-btn {
    width: 100%;
    padding: 7px 12px;
    background: var(--nord2);
    border: 1px solid var(--nord8);
    color: var(--nord8);
    border-radius: 3px;
    font-size: 12px;
    font-weight: bold;
  }

  .select-playing-btn:hover {
    background: var(--nord8);
    color: var(--nord0);
  }

  .inspector-form {
    flex: 1;
    overflow-y: auto;
    padding: 12px;
    display: flex;
    flex-direction: column;
    gap: 12px;
  }

  .artwork-section {
    display: flex;
    gap: 10px;
    align-items: center;
    background: var(--nord0);
    padding: 8px;
    border: 1px solid var(--nord3);
    border-radius: 4px;
  }

  .artwork-box {
    width: 68px;
    height: 68px;
    flex-shrink: 0;
    background: var(--nord1);
    border: 1px dashed var(--nord3);
    border-radius: 4px;
    display: flex;
    align-items: center;
    justify-content: center;
    overflow: hidden;
    cursor: pointer;
  }

  .artwork-box:hover,
  .artwork-box.drag-over {
    border-color: var(--nord8);
  }

  .art-preview-img {
    width: 100%;
    height: 100%;
    object-fit: cover;
  }

  .art-empty {
    display: flex;
    flex-direction: column;
    align-items: center;
    font-size: 10px;
    color: var(--nord3);
    text-align: center;
  }

  .artwork-controls {
    flex: 1;
    display: flex;
    flex-direction: column;
    gap: 6px;
    overflow: hidden;
  }

  .artwork-status {
    font-size: 11px;
  }

  .muted-note {
    color: var(--nord3);
  }

  .dirty-tag {
    color: var(--nord13);
    font-weight: bold;
  }

  .dirty-tag.warn {
    color: var(--nord11);
  }

  .art-btns {
    display: flex;
    gap: 6px;
  }

  .mini-btn {
    padding: 3px 8px;
    font-size: 11px;
    background: var(--nord2);
    border: 1px solid var(--nord3);
    border-radius: 2px;
    color: var(--nord4);
  }

  .mini-btn:hover {
    border-color: var(--nord8);
    color: var(--nord6);
  }

  .mini-btn.danger:hover {
    border-color: var(--nord11);
    color: var(--nord11);
  }

  .field-group {
    display: flex;
    flex-direction: column;
    gap: 4px;
  }

  .field-label-row {
    display: flex;
    justify-content: space-between;
    align-items: center;
  }

  .field-label-row label {
    font-size: 11px;
    font-weight: bold;
    color: var(--nord4);
  }

  .dirty-badge {
    font-size: 10px;
    color: var(--nord13);
    background: color-mix(in srgb, var(--nord13) 15%, transparent);
    padding: 0 5px;
    border-radius: 2px;
  }

  .field-group input[type="text"],
  .field-group input[type="number"] {
    width: 100%;
    background: var(--nord0);
    border: 1px solid var(--nord3);
    border-radius: 3px;
    padding: 5px 8px;
    font-size: 12px;
    color: var(--nord6);
  }

  .field-group input:disabled {
    opacity: 0.5;
    cursor: not-allowed;
  }

  .checkbox-option {
    display: flex;
    align-items: center;
    gap: 6px;
    font-size: 11px;
    color: var(--nord4);
    cursor: pointer;
    margin-top: 2px;
  }

  .inline-helper-btn {
    align-self: flex-start;
    font-size: 10px;
    color: var(--nord8);
    padding: 2px 4px;
    background: color-mix(in srgb, var(--nord8) 10%, transparent);
    border-radius: 2px;
  }

  .inline-helper-btn:hover {
    background: color-mix(in srgb, var(--nord8) 25%, transparent);
  }

  .track-num-options {
    display: flex;
    flex-direction: column;
    gap: 2px;
  }

  .status-banner {
    padding: 6px 10px;
    border-radius: 3px;
    font-size: 11px;
    background: color-mix(in srgb, var(--nord14) 20%, transparent);
    color: var(--nord14);
    border: 1px solid var(--nord14);
    white-space: pre-line;
    overflow-wrap: anywhere;
  }

  .status-banner.error {
    background: color-mix(in srgb, var(--nord11) 20%, transparent);
    color: var(--nord11);
    border-color: var(--nord11);
  }

  .form-actions {
    display: flex;
    gap: 8px;
    margin-top: 2px;
  }

  .save-btn {
    flex: 1;
    padding: 7px 12px;
    background: var(--nord8);
    color: var(--nord0);
    font-weight: bold;
    border-radius: 3px;
    font-size: 12px;
  }

  .save-btn:hover:not(:disabled) {
    background: var(--nord7);
  }

  .save-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .reset-btn {
    padding: 7px 10px;
    background: var(--nord2);
    color: var(--nord4);
    border: 1px solid var(--nord3);
    border-radius: 3px;
    font-size: 11px;
  }

  .reset-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .file-meta-box {
    margin-top: 4px;
    padding: 8px;
    background: var(--nord0);
    border: 1px solid var(--nord3);
    border-radius: 3px;
    display: flex;
    flex-direction: column;
    gap: 4px;
    font-size: 11px;
  }

  .meta-row {
    display: flex;
    gap: 6px;
  }

  .meta-key {
    color: var(--nord3);
    flex-shrink: 0;
  }

  .meta-val {
    color: var(--nord4);
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .meta-val.path {
    font-size: 10px;
    color: var(--nord3);
  }

  .selected-list-section {
    display: flex;
    flex-direction: column;
    border: 1px solid var(--nord3);
    border-radius: 3px;
    background: var(--nord0);
    overflow: hidden;
  }

  .selected-list-header {
    padding: 5px 8px;
    background: var(--nord2);
    font-size: 11px;
    font-weight: bold;
    color: var(--nord4);
    display: flex;
    justify-content: space-between;
  }

  .selected-list {
    max-height: 150px;
    overflow-y: auto;
    display: flex;
    flex-direction: column;
  }

  .selected-row {
    display: flex;
    align-items: center;
    padding: 4px 8px;
    font-size: 11px;
    border-bottom: 1px solid color-mix(in srgb, var(--nord3) 25%, transparent);
    gap: 4px;
  }

  .sel-idx {
    color: var(--nord8);
    width: 20px;
    flex-shrink: 0;
  }

  .sel-name {
    flex: 1;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--nord4);
  }

  .sel-order-btns {
    display: flex;
    gap: 2px;
  }

  .ord-btn {
    padding: 0 4px;
    font-size: 11px;
    color: var(--nord4);
    background: var(--nord1);
    border: 1px solid var(--nord3);
    border-radius: 2px;
  }

  .ord-btn:disabled {
    opacity: 0.3;
  }

  .ord-btn.remove:hover {
    color: var(--nord11);
    border-color: var(--nord11);
  }
</style>
