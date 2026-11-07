<script lang="ts">
  import { FONT_SCALES } from "../fontScale";
  import { playerState } from "../playerState.svelte";
  import type { ThemeSetting } from "../types";

  const THEME_OPTIONS: { value: ThemeSetting; label: string }[] = [
    { value: "guif", label: "guif 連動" },
    { value: "nord", label: "Nord" },
    { value: "default", label: "Default" },
    { value: "monokai", label: "Monokai" },
    { value: "catppuccin", label: "Catppuccin" },
    { value: "solarized", label: "Solarized" },
    { value: "light", label: "Light" },
  ];

  let cardEl = $state<HTMLDivElement | null>(null);

  // Keyboard users start inside the dialog and return to where they were
  $effect(() => {
    if (!cardEl) return;
    const opener = document.activeElement as HTMLElement | null;
    cardEl.querySelector<HTMLElement>("input:checked")?.focus();
    return () => opener?.focus();
  });

  function close() {
    playerState.showSettings = false;
  }
</script>

{#if playerState.showSettings}
  <div
    class="modal-backdrop"
    role="dialog"
    aria-modal="true"
    aria-label="設定"
    tabindex="-1"
    onclick={close}
    onkeydown={(e) => {
      if (e.key === "Escape") {
        // Handled here: the window's Esc (such as closing an album) must not run too
        e.stopPropagation();
        close();
      }
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
        <h3>設定</h3>
        <button class="close-btn" title="閉じる (Esc)" onclick={close}>×</button>
      </div>

      <fieldset>
        <legend>テーマ</legend>
        <div class="options">
          {#each THEME_OPTIONS as option}
            <label class="option">
              <input
                type="radio"
                name="theme"
                value={option.value}
                checked={playerState.themeSetting === option.value}
                onchange={() => playerState.setTheme(option.value)}
              />
              {option.label}
            </label>
          {/each}
        </div>
      </fieldset>

      <fieldset>
        <legend>文字の大きさ</legend>
        <div class="options">
          {#each FONT_SCALES as size}
            <label class="option">
              <input
                type="radio"
                name="font-scale"
                value={size.id}
                checked={playerState.fontScale === size.id}
                onchange={() => playerState.setFontScale(size.id)}
              />
              {size.label}
            </label>
          {/each}
        </div>
        <p class="hint">Ctrl + = で大きく、Ctrl + - で小さく、Ctrl + 0 で標準に戻します。</p>
      </fieldset>
    </div>
  </div>
{/if}

<style>
  .modal-backdrop {
    position: fixed;
    inset: 0;
    z-index: 100;
    display: flex;
    align-items: center;
    justify-content: center;
    background: color-mix(in srgb, var(--nord0) 70%, transparent);
  }

  .modal-card {
    width: min(420px, calc(100vw - 32px));
    max-height: calc(100vh - 32px);
    overflow-y: auto;
    padding: 16px;
    background: var(--nord1);
    border: 1px solid var(--nord3);
    border-radius: 4px;
    outline: none;
  }

  .modal-header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 8px;
  }

  h3 {
    margin: 0;
    font-size: 14px;
    color: var(--nord6);
  }

  .close-btn {
    padding: 0 6px;
    color: var(--nord4);
    cursor: pointer;
  }

  fieldset {
    margin: 12px 0 0;
    padding: 8px 12px 12px;
    border: 1px solid var(--nord3);
    border-radius: 3px;
  }

  legend {
    padding: 0 4px;
    font-size: 12px;
    color: var(--nord8);
  }

  .options {
    display: flex;
    flex-wrap: wrap;
    gap: 6px 14px;
  }

  .option {
    display: flex;
    align-items: center;
    gap: 4px;
    font-size: 12px;
    color: var(--nord4);
    cursor: pointer;
  }

  .option input {
    accent-color: var(--nord8);
  }

  .hint {
    margin: 10px 0 0;
    font-size: 11px;
    color: var(--ui-text-muted);
  }
</style>
