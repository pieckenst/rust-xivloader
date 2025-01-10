<script lang="ts">
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { X, Minus, Square, Maximize2 } from 'lucide-svelte';
  import { onMount } from 'svelte';

  let isMaximized = false;
  let isResizing = false;
  let resizeTimeout: number;

  export let title = "XIVloader";
  export let centerTitle = true;
  export let showMinimize = true;
  export let showMaximize = true;
  export let accentColor: string | undefined = undefined;
  export let leadingColor: string | undefined = undefined;
  export let trailingColor: string | undefined = undefined;

  onMount(async () => {
    const window = getCurrentWindow();
    isMaximized = await window.isMaximized();
    document.body.classList.add('titlebar-enabled');

    window.listen('tauri://resize', () => {
      isResizing = true;
      clearTimeout(resizeTimeout);
      resizeTimeout = setTimeout(() => {
        isResizing = false;
      }, 600);
    });

    window.listen('tauri://move', () => {
      isResizing = true;
      clearTimeout(resizeTimeout);
      resizeTimeout = setTimeout(() => {
        isResizing = false;
      }, 600);
    });

    window.listen('tauri://maximize', async () => {
      isMaximized = await window.isMaximized();
    });

    window.listen('tauri://unmaximize', () => {
      isMaximized = false;
    });
  });

  async function minimize() {
    const window = getCurrentWindow();
    await window.minimize();
  }

  async function maximize() {
    const window = getCurrentWindow();
    if (isMaximized) {
      await window.unmaximize();
    } else {
      await window.maximize();
    }
    isMaximized = await window.isMaximized();
  }

  async function close() {
    const window = getCurrentWindow();
    await window.close();
  }

  function handleDoubleClick(event: MouseEvent) {
    if ((event.target as HTMLElement).closest('.window-controls')) return;
    if (showMaximize) maximize();
  }
</script>

<div 
  class="titlebar" 
  class:maximized={isMaximized}
  class:resizing={isResizing}
  class:has-accent={accentColor}
  style:--accent-color={accentColor}
  data-tauri-drag-region 
  on:dblclick={handleDoubleClick}
  role="presentation"
  aria-label="Window titlebar"
>
  <div 
    class="titlebar-content" 
    class:has-centered-title={centerTitle}
    class:has-leading-title={!centerTitle}
    class:has-minimize={showMinimize}
    class:has-maximize={showMaximize}
    data-tauri-drag-region
    role="presentation"
  >
    <div 
      class="titlebar-leading" 
      class:has-color={leadingColor}
      style:--leading-color={leadingColor}
      data-tauri-drag-region
      role="presentation"
    >
      {#if !centerTitle}
        <span class="title" class:visible={!centerTitle}>{title}</span>
      {/if}
      <slot name="leading" />
    </div>

    <div 
      class="titlebar-center"
      class:has-color={accentColor} 
      data-tauri-drag-region
      role="presentation"
    >
      {#if centerTitle}
        <span class="title" class:visible={centerTitle}>{title}</span>
      {/if}
      <slot name="center" />
    </div>

    <div 
      class="titlebar-trailing"
      class:has-color={trailingColor}
      style:--trailing-color={trailingColor}
      data-tauri-drag-region
      role="presentation"
    >
      <slot name="trailing" />
      <div class="window-controls" role="group" aria-label="Window controls">
        <div class="window-controls-group">
          {#if showMinimize}
            <button on:click={minimize} class="window-control minimize" title="Minimize">
              <Minus class="h-3.5 w-3.5" strokeWidth={2.5} />
            </button>
          {/if}
          {#if showMaximize}
            <button on:click={maximize} class="window-control maximize" title="Maximize">
              {#if isMaximized}
                <Square class="h-3.5 w-3.5" strokeWidth={2.5} />
              {:else}
                <Maximize2 class="h-3.5 w-3.5" strokeWidth={2.5} />
              {/if}
            </button>
          {/if}
          <button on:click={close} class="window-control close" title="Close">
            <X class="h-3.5 w-3.5" strokeWidth={2.5} />
          </button>
        </div>
      </div>
    </div>
  </div>
</div>

<style>
  .titlebar {
    height: 47px;
    background: var(--background);
    user-select: none;
    position: fixed;
    top: 0;
    left: 0;
    right: 0;
    z-index: 50;
    border-bottom: 1px solid var(--border);
    border-top-left-radius: 12px;
    border-top-right-radius: 12px;
    transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  .titlebar.maximized {
    border-top-left-radius: 0;
    border-top-right-radius: 0;
  }

  .titlebar.resizing {
    transition: none;
  }

  .titlebar-content {
    display: flex;
    align-items: stretch;
    height: 100%;
    padding: 0;
    position: relative;
  }

  .titlebar-leading {
    flex: 1;
    display: flex;
    align-items: center;
    height: 100%;
    min-width: 120px;
    padding-left: 12px;
    position: relative;
    transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  .titlebar-leading.has-color {
    background-color: var(--leading-color);
  }

  .titlebar-center {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
    padding: 0 16px;
  }

  .titlebar-center.has-color {
    background-color: var(--accent-color);
  }

  .titlebar-trailing {
    flex: 1;
    display: flex;
    align-items: center;
    justify-content: flex-end;
    height: 100%;
    min-width: 120px;
    position: relative;
    transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  .titlebar-trailing.has-color {
    background-color: var(--trailing-color);
  }

  /* Title styles and animations */
  .title {
    font-size: 13px;
    color: var(--foreground);
    font-weight: 600;
    letter-spacing: -0.1px;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
    max-width: 260px;
    opacity: 0;
    position: absolute;
    left: 0;
    transform: translateY(5px);
    transition: all 300ms cubic-bezier(0.4, 0, 0.2, 1);
    pointer-events: none;
  }

  .titlebar-center .title {
    position: relative;
    left: unset;
    opacity: 0;
    transform: translateY(5px);
  }

  .titlebar-leading .title {
    position: relative;
    left: unset;
    opacity: 0;
    transform: translateY(5px);
  }

  .title.visible {
    opacity: 1;
    transform: translateY(0);
  }

  /* Animate title container positions */
  .titlebar-center {
    position: absolute;
    left: 50%;
    transform: translateX(-50%);
    height: 100%;
    display: flex;
    align-items: center;
    justify-content: center;
    transition: all 300ms cubic-bezier(0.4, 0, 0.2, 1);
    padding: 0 16px;
    opacity: 0;
  }

  .titlebar-leading {
    flex: 1;
    display: flex;
    align-items: center;
    height: 100%;
    min-width: 120px;
    padding-left: 12px;
    position: relative;
    transition: all 300ms cubic-bezier(0.4, 0, 0.2, 1);
    opacity: 0;
  }

  .has-centered-title .titlebar-center {
    opacity: 1;
  }

  .has-leading-title .titlebar-leading {
    opacity: 1;
  }

  /* Keep essential positioning styles */
  .titlebar-content.has-centered-title .titlebar-center {
    margin-right: var(--controls-width);
    transition: all 300ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  .titlebar-content.has-centered-title.has-minimize.has-maximize {
    --controls-width: 110px;
  }

  .titlebar-content.has-centered-title.has-minimize:not(.has-maximize),
  .titlebar-content.has-centered-title.has-maximize:not(.has-minimize) {
    --controls-width: 80px;
  }

  .titlebar-content.has-centered-title:not(.has-minimize):not(.has-maximize) {
    --controls-width: 50px;
  }

  /* Window controls styles */
  .window-controls {
    display: flex;
    height: 100%;
    align-items: center;
    padding-top: 5px;
    transition: all 300ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  .window-controls-group {
    display: flex;
    gap: 3px;
    padding: 0 8px;
    height: 32px;
    align-items: center;
    transition: all 300ms cubic-bezier(0.4, 0, 0.2, 1);
  }

  .window-control {
    width: 24px;
    height: 24px;
    border: none;
    background: rgba(255, 255, 255, 0.08);
    outline: none;
    padding: 0;
    display: flex;
    align-items: center;
    justify-content: center;
    color: var(--muted-foreground);
    transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
    border-radius: 50%;
    opacity: 0;
    transform: scale(0.9);
  }

  /* Button animations */
  .window-controls-group .minimize {
    animation: buttonIn 300ms cubic-bezier(0.4, 0, 0.2, 1) forwards;
  }

  .window-controls-group .maximize {
    animation: buttonIn 300ms cubic-bezier(0.4, 0, 0.2, 1) forwards;
  }

  /* When buttons are disabled/removed */
  .titlebar-content:not(.has-minimize) .minimize {
    animation: buttonOut 300ms cubic-bezier(0.4, 0, 0.2, 1) forwards;
  }

  .titlebar-content:not(.has-maximize) .maximize {
    animation: buttonOut 300ms cubic-bezier(0.4, 0, 0.2, 1) forwards;
  }

  @keyframes buttonIn {
    0% {
      opacity: 0;
      transform: scale(0.8) translateY(5px);
    }
    60% {
      transform: scale(1.05) translateY(-2px);
    }
    100% {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
  }

  @keyframes buttonOut {
    0% {
      opacity: 1;
      transform: scale(1) translateY(0);
    }
    40% {
      transform: scale(1.05) translateY(-2px);
    }
    100% {
      opacity: 0;
      transform: scale(0.8) translateY(5px);
    }
  }

  .window-control:hover {
    background: rgba(255, 255, 255, 0.12);
    color: var(--foreground);
    transform: scale(1.05);
  }

  .window-control:active {
    background: rgba(255, 255, 255, 0.16);
    transform: scale(0.95);
  }

  .close {
    background: rgba(246, 97, 81, 0.08);
    color: rgb(246, 97, 81);
    animation: buttonIn 300ms cubic-bezier(0.4, 0, 0.2, 1) forwards;
  }

  .close:hover {
    background: rgb(246, 97, 81);
    color: white;
    transform: scale(1.05);
  }

  .close:active {
    background: rgb(220, 87, 73);
    transform: scale(0.95);
  }

  [data-tauri-drag-region] {
    cursor: move;
  }

  :global(body.titlebar-enabled) {
    border-radius: 12px;
    overflow: hidden;
  }

  :global(body.titlebar-enabled .titlebar) {
    -webkit-app-region: drag;
    background: linear-gradient(180deg, 
      color-mix(in srgb, var(--background) 98%, var(--foreground)) 0%,
      var(--background) 100%
    );
  }

  :global(body.titlebar-enabled .window-control) {
    -webkit-app-region: no-drag;
  }

  /* Hide window size indicator and overlays */
  :global(body.titlebar-enabled::before),
  :global(body.titlebar-enabled::after),
  :global(.window-size-indicator),
  :global(.window-controls-overlay),
  :global(.window-resize-indicator) {
    display: none !important;
    opacity: 0 !important;
    pointer-events: none !important;
    visibility: hidden !important;
  }

  /* Ensure controls stay on top and prevent overlays */
  .window-controls {
    position: relative;
    z-index: 9999;
    isolation: isolate;
  }

  .window-controls-group {
    position: relative;
    z-index: 9999;
  }

  @media (prefers-color-scheme: dark) {
    :global(body.titlebar-enabled .titlebar) {
      background: linear-gradient(180deg,
        color-mix(in srgb, var(--background) 95%, var(--foreground)) 0%,
        var(--background) 100%
      );
    }
  }
</style> 