<script lang="ts">
  import { onMount } from "svelte";
  import AddToPlaylistModal from "../lib/components/AddToPlaylistModal.svelte";
  import ArtworkGridView from "../lib/components/ArtworkGridView.svelte";
  import ErrorNotice from "../lib/components/ErrorNotice.svelte";
  import SettingsDialog from "../lib/components/SettingsDialog.svelte";
  import FolderListView from "../lib/components/FolderListView.svelte";
  import NowPlayingBar from "../lib/components/NowPlayingBar.svelte";
  import PathHeader from "../lib/components/PathHeader.svelte";
  import PlayerDetailDialog from "../lib/components/PlayerDetailDialog.svelte";
  import PlaylistView from "../lib/components/PlaylistView.svelte";
  import PropertyInspector from "../lib/components/PropertyInspector.svelte";
  import { focusActiveNavItem } from "../lib/listNavigation";
  import { playerState } from "../lib/playerState.svelte";
  import type { AudioTrack } from "../lib/types";

  let inspectorRef = $state<{ saveChanges: () => Promise<void> } | null>(null);

  onMount(() => {
    playerState.init();

    function handleKeydown(e: KeyboardEvent) {
      const activeEl = document.activeElement;
      const isTextInput =
        (activeEl instanceof HTMLInputElement &&
          activeEl.type !== "checkbox" &&
          activeEl.type !== "range") ||
        activeEl instanceof HTMLTextAreaElement;

      // Global shortcuts
      if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "s") {
        e.preventDefault();
        inspectorRef?.saveChanges();
      } else if ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === "i") {
        e.preventDefault();
        playerState.showInspector = !playerState.showInspector;
      } else if (
        (e.ctrlKey || e.metaKey) &&
        e.key.toLowerCase() === "a" &&
        !isTextInput
      ) {
        e.preventDefault();
        if (playerState.viewMode === "FOLDER") {
          const tracks = (playerState.folderContent?.items ?? [])
            .filter((i) => !i.is_directory && i.track !== null)
            .map((i) => i.track as AudioTrack);
          playerState.selectAllTracks(tracks);
          playerState.showInspector = true;
        } else if (playerState.viewMode === "PLAYLIST") {
          playerState.selectAllTracks(playerState.playlistTracks);
          playerState.showInspector = true;
        }
      } else if (
        e.key.toLowerCase() === "b" &&
        !e.ctrlKey &&
        !e.altKey &&
        !e.metaKey &&
        !isTextInput &&
        playerState.playback.current_track
      ) {
        e.preventDefault();
        playerState.addBookmark();
      } else if (
        (e.key === "[" || e.key === "]") &&
        !e.ctrlKey &&
        !e.altKey &&
        !e.metaKey &&
        !isTextInput
      ) {
        e.preventDefault();
        playerState.stepPlaybackSpeed(e.key === "]" ? 1 : -1);
      } else if (e.key === " " && !e.ctrlKey && !isTextInput) {
        e.preventDefault();
        playerState.togglePlayPause();
      } else if (
        (e.key === "Backspace" || (e.altKey && e.key === "ArrowUp")) &&
        (playerState.viewMode === "FOLDER" || playerState.viewMode === "ARTWORK") &&
        !isTextInput
      ) {
        e.preventDefault();
        if (playerState.openedAlbum) {
          // In the album view, back from an album's tracks to the grid first
          playerState.closeAlbum();
        } else {
          playerState.navigateUp();
        }
      } else if (
        (e.key === "ArrowDown" || e.key === "ArrowUp") &&
        activeEl === document.body &&
        !playerState.showDetailDialog &&
        !playerState.playlistModalTrack
      ) {
        e.preventDefault();
        focusActiveNavItem();
      } else if (e.ctrlKey && e.key === "ArrowRight" && !isTextInput) {
        e.preventDefault();
        playerState.playNext();
      } else if (e.ctrlKey && e.key === "ArrowLeft" && !isTextInput) {
        e.preventDefault();
        playerState.playPrevious();
      } else if (e.altKey && e.key === "ArrowRight" && !isTextInput) {
        e.preventDefault();
        playerState.seek(playerState.playback.position_ms + 5000);
      } else if (e.altKey && e.key === "ArrowLeft" && !isTextInput) {
        e.preventDefault();
        playerState.seek(Math.max(0, playerState.playback.position_ms - 5000));
      } else if (e.ctrlKey && ["+", "=", ";"].includes(e.key)) {
        e.preventDefault();
        playerState.stepFontScale(1);
      } else if (e.ctrlKey && e.key === "-") {
        e.preventDefault();
        playerState.stepFontScale(-1);
      } else if (e.ctrlKey && e.key === "0") {
        e.preventDefault();
        playerState.setFontScale("normal");
      } else if (e.ctrlKey && e.key === "1") {
        e.preventDefault();
        playerState.setViewMode("FOLDER");
      } else if (e.ctrlKey && e.key === "2") {
        e.preventDefault();
        playerState.setViewMode("ARTWORK");
      } else if (e.ctrlKey && e.key === "3") {
        e.preventDefault();
        playerState.setViewMode("PLAYLIST");
      } else if (e.key === "Escape") {
        if (playerState.showSettings) {
          playerState.showSettings = false;
        } else if (playerState.showDetailDialog) {
          playerState.showDetailDialog = false;
        } else if (playerState.playlistModalTrack) {
          playerState.playlistModalTrack = null;
        } else if (playerState.errorMessage) {
          playerState.clearError();
        } else if (playerState.openedAlbum && playerState.viewMode === "ARTWORK") {
          playerState.closeAlbum();
        } else {
          // Back to the list from the inspector or the header
          focusActiveNavItem();
        }
      }
    }

    window.addEventListener("keydown", handleKeydown);
    return () => {
      window.removeEventListener("keydown", handleKeydown);
    };
  });
</script>

<div class="app-layout">
  <!-- Path & Navigation Header -->
  <PathHeader />

  <!-- Main Content Area + Right Property Inspector (Pattern B) -->
  <div class="content-area">
    <div class="primary-view">
      {#if playerState.viewMode === "FOLDER"}
        <FolderListView />
      {:else if playerState.viewMode === "ARTWORK"}
        <ArtworkGridView />
      {:else if playerState.viewMode === "PLAYLIST"}
        <PlaylistView />
      {/if}
    </div>

    <PropertyInspector bind:this={inspectorRef} />
  </div>

  <!-- Now Playing Bar -->
  <NowPlayingBar />

  <ErrorNotice />
  <SettingsDialog />

  <!-- Modals -->
  <PlayerDetailDialog />
  <AddToPlaylistModal />
</div>

<style>
  .app-layout {
    display: flex;
    flex-direction: column;
    width: 100vw;
    height: 100vh;
    overflow: hidden;
    background-color: var(--nord0);
  }

  .content-area {
    flex: 1;
    display: flex;
    overflow: hidden;
  }

  .primary-view {
    flex: 1;
    display: flex;
    overflow: hidden;
  }
</style>
