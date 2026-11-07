<script lang="ts">
  import { tick } from "svelte";
  import {
    focusNavItem,
    hasCommandModifier,
    nextListIndex,
    visibleItemCount,
  } from "../listNavigation";
  import { formatTime, playerState } from "../playerState.svelte";
  import type { Bookmark } from "../types";

  let listEl = $state<HTMLDivElement | null>(null);
  let activeIndex = $state(0);
  let activeRow = $derived(Math.min(activeIndex, playerState.bookmarks.length - 1));

  async function remove(bookmark: Bookmark) {
    await playerState.deleteBookmark(bookmark.file_path);
    await tick();
    focusNavItem(listEl, activeRow);
  }

  function onKeydown(e: KeyboardEvent, bookmark: Bookmark, index: number) {
    if (e.key === "Enter") {
      if (e.target !== e.currentTarget) return;
      playerState.playBookmark(bookmark);
      return;
    }
    if (hasCommandModifier(e) || e.shiftKey) return;
    if (e.key === "Delete") {
      e.preventDefault();
      remove(bookmark);
      return;
    }
    const next = nextListIndex(e.key, index, playerState.bookmarks.length, {
      pageSize: visibleItemCount(listEl),
    });
    if (next === null) return;
    e.preventDefault();
    activeIndex = next;
    focusNavItem(listEl, next);
  }
</script>

<div class="bookmark-view">
  <div class="content-header">
    <h2>★ しおり</h2>
    <span class="count">{playerState.bookmarks.length} 件</span>
  </div>

  {#if playerState.bookmarks.length === 0}
    <div class="empty">
      しおりがありません。<br />
      再生中に詳細プレーヤーの [★ しおり] か B キーで、今の位置に挟めます。
    </div>
  {:else}
    <div class="list" bind:this={listEl}>
      {#each playerState.bookmarks as bookmark, index (bookmark.file_path)}
        <div
          class="row"
          role="button"
          data-nav-item
          tabindex={index === activeRow ? 0 : -1}
          title="この位置から再生 (Enter)"
          onclick={() => playerState.playBookmark(bookmark)}
          onfocus={() => (activeIndex = index)}
          onkeydown={(e) => onKeydown(e, bookmark, index)}
        >
          <span class="position">{formatTime(bookmark.position_ms)}</span>
          <span class="title">{bookmark.title}</span>
          <span class="album">{bookmark.album}</span>
          <button
            class="delete-btn"
            tabindex="-1"
            title="しおりを削除 (Delete)"
            onclick={(e) => {
              e.stopPropagation();
              remove(bookmark);
            }}
          >
            ×
          </button>
        </div>
      {/each}
    </div>
  {/if}
</div>

<style>
  .bookmark-view {
    flex: 1;
    display: flex;
    flex-direction: column;
    overflow: hidden;
  }

  .content-header {
    display: flex;
    align-items: baseline;
    gap: 12px;
    padding: 16px;
    border-bottom: 1px solid var(--nord3);
  }

  h2 {
    margin: 0;
    font-size: 16px;
    color: var(--nord6);
  }

  .count,
  .empty {
    color: var(--ui-text-muted);
    font-size: 12px;
  }

  .empty {
    padding: 60px 20px;
    text-align: center;
    line-height: 1.8;
  }

  .list {
    overflow-y: auto;
  }

  .row {
    display: flex;
    align-items: center;
    gap: 12px;
    padding: 8px 16px;
    border-bottom: 1px solid color-mix(in srgb, var(--nord3) 25%, transparent);
    font-size: 12px;
    color: var(--nord4);
    cursor: pointer;
    outline: none;
  }

  .row:hover {
    background: var(--nord2);
  }

  .position {
    flex-shrink: 0;
    width: 48px;
    color: var(--nord13);
  }

  .title {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--nord6);
  }

  .album {
    flex: 1;
    min-width: 0;
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
    color: var(--ui-text-muted);
  }

  .delete-btn {
    flex-shrink: 0;
    padding: 0 6px;
    color: var(--ui-text-muted);
    cursor: pointer;
  }

  .delete-btn:hover {
    color: var(--nord11);
  }
</style>
