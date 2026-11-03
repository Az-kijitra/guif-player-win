<script lang="ts">
  import { playerState } from "../playerState.svelte";
  import type { ThemeSetting } from "../types";

  const THEME_OPTIONS: { value: ThemeSetting; label: string }[] = [
    { value: "guif", label: "🎨 guif連動" },
    { value: "nord", label: "🎨 Nord" },
    { value: "default", label: "🎨 Default" },
    { value: "monokai", label: "🎨 Monokai" },
    { value: "catppuccin", label: "🎨 Catppuccin" },
    { value: "solarized", label: "🎨 Solarized" },
    { value: "light", label: "🎨 Light" },
  ];

  function onThemeChange(e: Event) {
    const val = (e.target as HTMLSelectElement).value as ThemeSetting;
    playerState.setTheme(val);
  }

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
</script>

<header class="path-header">
  <div class="top-row">
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
    <div class="mode-tabs">
      <button
        class="tab-btn"
        class:active={playerState.viewMode === "FOLDER"}
        onclick={() => playerState.setViewMode("FOLDER")}
      >
        📁 フォルダ
      </button>
      <button
        class="tab-btn"
        class:active={playerState.viewMode === "ARTWORK"}
        onclick={() => playerState.setViewMode("ARTWORK")}
      >
        🖼 アルバム
      </button>
      <button
        class="tab-btn"
        class:active={playerState.viewMode === "PLAYLIST"}
        onclick={() => playerState.setViewMode("PLAYLIST")}
      >
        📜 プレイリスト
      </button>
      <button
        class="tab-btn inspector-toggle"
        class:active={playerState.showInspector}
        onclick={() => (playerState.showInspector = !playerState.showInspector)}
        title="右サイドのプロパティパネルを開閉 (Ctrl+I)"
      >
        🏷 プロパティ
        {#if playerState.selectedTracks.length > 0}
          <span class="sel-count">{playerState.selectedTracks.length}</span>
        {/if}
      </button>

      <select
        class="theme-select"
        value={playerState.themeSetting}
        onchange={onThemeChange}
        title={`カラーテーマ切替 (現在: ${playerState.resolvedTheme})`}
      >
        {#each THEME_OPTIONS as opt}
          <option value={opt.value}>{opt.label}</option>
        {/each}
      </select>
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
        else playerState.navigateTo(playerState.currentPath);
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
    padding: 3px 10px;
    background: var(--nord2);
    border: 1px solid var(--nord3);
    color: var(--nord4);
    border-radius: 2px;
    font-size: 12px;
  }

  .tab-btn.active {
    background: var(--nord9);
    color: var(--nord0);
    border-color: var(--nord8);
    font-weight: bold;
  }

  .theme-select {
    margin-left: 4px;
    padding: 2px 6px;
    background: var(--nord2);
    border: 1px solid var(--nord3);
    color: var(--nord4);
    border-radius: 2px;
    font-family: inherit;
    font-size: 12px;
    cursor: pointer;
    outline: none;
  }

  .theme-select:hover,
  .theme-select:focus {
    border-color: var(--nord8);
    color: var(--nord6);
  }

  .theme-select option {
    background: var(--nord1);
    color: var(--nord4);
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
