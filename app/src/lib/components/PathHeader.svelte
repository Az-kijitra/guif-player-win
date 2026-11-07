<script lang="ts">
  import { tick } from "svelte";
  import { playerState } from "../playerState.svelte";

  function splitPath(fullPath: string): { name: string; path: string }[] {
    if (!fullPath) return [];
    const parts = fullPath.split(/[/\\]/).filter(Boolean);
    const result: { name: string; path: string }[] = [];
    let accumulated = "";

    // Windows drive handling
    if (fullPath.includes(":\\")) {
      const drive = fullPath.substring(0, 3);
      result.push({ name: drive, path: drive });
      accumulated = drive;
      for (let i = 1; i < parts.length; i++) {
        accumulated += parts[i] + "\\";
        result.push({ name: parts[i], path: accumulated });
      }
    } else {
      for (const part of parts) {
        accumulated += "/" + part;
        result.push({ name: part, path: accumulated });
      }
    }
    return result;
  }

  let segments = $derived(splitPath(playerState.currentPath));

  // Narrow windows and large text show the tabs as icons only, rather than wrapping their names.
  // The width they need depends on the number of drives, so it is measured, not fixed.
  let topRowEl = $state<HTMLDivElement | null>(null);
  let compact = $state(false);

  async function fitTabs() {
    if (!topRowEl) return;
    compact = false;
    await tick();
    compact = topRowEl.scrollWidth > topRowEl.clientWidth;
  }

  $effect(() => {
    if (!topRowEl) return;
    const observer = new ResizeObserver(() => fitTabs());
    observer.observe(topRowEl);
    return () => observer.disconnect();
  });

  // More drives need more room
  $effect(() => {
    void playerState.drives.length;
    fitTabs();
  });
</script>

<header class="path-header">
  <div class="top-row" bind:this={topRowEl}>
    <!-- Drives -->
    <div class="drives">
      {#each playerState.drives as drive}
        <button
          class="drive-btn"
          class:active={playerState.currentPath.startsWith(drive)}
          onclick={() => playerState.navigateTo(drive)}
        >
          {drive}
        </button>
      {/each}
    </div>

    <!-- View Mode Switcher -->
    <div class="mode-tabs" class:compact>
      <button
        class="tab-btn"
        class:active={playerState.viewMode === "FOLDER"}
        title="フォルダ (Ctrl+1)"
        onclick={() => playerState.setViewMode("FOLDER")}
      >
        📁<span class="label">フォルダ</span>
      </button>
      <button
        class="tab-btn"
        class:active={playerState.viewMode === "ARTWORK"}
        title="アルバム (Ctrl+2)"
        onclick={() => playerState.setViewMode("ARTWORK")}
      >
        🖼<span class="label">アルバム</span>
      </button>
      <button
        class="tab-btn"
        class:active={playerState.viewMode === "PLAYLIST"}
        title="プレイリスト (Ctrl+3)"
        onclick={() => playerState.setViewMode("PLAYLIST")}
      >
        📜<span class="label">プレイリスト</span>
      </button>
      <button
        class="tab-btn inspector-toggle"
        class:active={playerState.showInspector}
        onclick={() => (playerState.showInspector = !playerState.showInspector)}
        title="右サイドのプロパティパネルを開閉 (Ctrl+I)"
      >
        🏷<span class="label">プロパティ</span>
        {#if playerState.selectedTracks.length > 0}
          <span class="sel-count">{playerState.selectedTracks.length}</span>
        {/if}
      </button>

      <button
        class="tab-btn"
        title="テーマと文字の大きさ"
        onclick={() => (playerState.showSettings = true)}
      >
        ⚙<span class="label">設定</span>
      </button>
    </div>
  </div>

  <div class="bottom-row">
    <button
      class="nav-btn"
      disabled={!playerState.folderContent?.parent_path}
      onclick={() => playerState.navigateUp()}
      title="親フォルダへ移動"
    >
      ↑ 上へ
    </button>

    <div class="breadcrumbs">
      {#each segments as seg, idx}
        <button class="breadcrumb-item" onclick={() => playerState.navigateTo(seg.path)}>
          {seg.name}
        </button>
        {#if idx < segments.length - 1}
          <span class="sep">/</span>
        {/if}
      {/each}
    </div>

    {#if playerState.isLoadingFolder || playerState.isLoadingAlbums}
      <span class="loading" role="status">読み込み中…</span>
    {/if}

    <button
      class="nav-btn refresh"
      onclick={() => {
        if (playerState.viewMode === "ARTWORK") playerState.loadAlbums();
        else if (playerState.viewMode === "PLAYLIST") playerState.loadPlaylists();
        else playerState.refreshFolder();
      }}
      title="更新"
    >
      🔄
    </button>
  </div>
</header>

<style>
  .loading {
    flex-shrink: 0;
    font-size: 11px;
    color: var(--nord4);
  }

  .path-header {
    background-color: var(--nord1);
    border-bottom: 1px solid var(--nord3);
    padding: 6px 12px;
    display: flex;
    flex-direction: column;
    gap: 6px;
  }

  .top-row, .bottom-row {
    display: flex;
    align-items: center;
    gap: 8px;
  }

  .drives {
    display: flex;
    gap: 4px;
  }

  .drive-btn {
    padding: 2px 8px;
    background: var(--nord2);
    border: 1px solid var(--nord3);
    color: var(--nord4);
    font-weight: bold;
    border-radius: 2px;
  }

  .drive-btn.active {
    background: var(--nord8);
    color: var(--nord0);
    border-color: var(--nord8);
  }

  .mode-tabs {
    margin-left: auto;
    display: flex;
    gap: 4px;
  }

  .tab-btn {
    display: flex;
    align-items: center;
    gap: 4px;
    white-space: nowrap;
    padding: 3px 10px;
    background: var(--nord2);
    border: 1px solid var(--nord3);
    color: var(--nord4);
    border-radius: 2px;
    font-size: 12px;
  }

  .mode-tabs.compact .label {
    display: none;
  }

  .mode-tabs.compact .tab-btn {
    padding: 3px 7px;
  }

  .tab-btn.active {
    background: var(--nord9);
    color: var(--nord0);
    border-color: var(--nord8);
    font-weight: bold;
  }

  .inspector-toggle {
    margin-left: 6px;
    display: flex;
    align-items: center;
    gap: 5px;
  }

  .sel-count {
    font-size: 10px;
    background: var(--nord8);
    color: var(--nord0);
    padding: 0 5px;
    border-radius: 8px;
    font-weight: bold;
  }

  .nav-btn {
    padding: 2px 8px;
    background: var(--nord2);
    border: 1px solid var(--nord3);
    color: var(--nord4);
    border-radius: 2px;
  }

  .nav-btn:disabled {
    opacity: 0.4;
    cursor: default;
  }

  .breadcrumbs {
    flex: 1;
    display: flex;
    align-items: center;
    gap: 4px;
    overflow-x: auto;
    white-space: nowrap;
    padding: 2px 4px;
    background: var(--nord0);
    border: 1px solid var(--nord3);
    border-radius: 2px;
  }

  .breadcrumb-item {
    padding: 1px 4px;
    color: var(--nord8);
    font-size: 12px;
  }

  .breadcrumb-item:hover {
    background: var(--nord2);
    text-decoration: underline;
  }

  .sep {
    color: var(--ui-text-muted);
  }

  .refresh {
    font-size: 13px;
  }
</style>
