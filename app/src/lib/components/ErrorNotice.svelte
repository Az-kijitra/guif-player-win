<script lang="ts">
  import { playerState } from "../playerState.svelte";
</script>

{#if playerState.errorMessage}
  <div class="error-notice" role="alert">
    <span class="message">{playerState.errorMessage}</span>
    <button
      class="close-btn"
      title="閉じる (Esc)"
      onclick={() => playerState.clearError()}
    >
      ×
    </button>
  </div>
{/if}

<style>
  .error-notice {
    position: fixed;
    right: 16px;
    bottom: 96px;
    z-index: 50;
    display: flex;
    align-items: flex-start;
    gap: 8px;
    max-width: min(520px, calc(100vw - 32px));
    padding: 8px 10px;
    font-size: 12px;
    color: var(--nord6);
    background: var(--nord1);
    border: 1px solid var(--nord11);
    border-left-width: 4px;
    border-radius: 3px;
  }

  .message {
    overflow-wrap: anywhere;
    white-space: pre-line;
  }

  .close-btn {
    flex-shrink: 0;
    padding: 0 4px;
    color: var(--nord4);
    cursor: pointer;
  }
</style>
