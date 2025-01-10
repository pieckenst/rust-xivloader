<script lang="ts">
  import '../app.postcss';
  import { Toaster } from 'svelte-sonner';
  import TitleBar from '$lib/components/TitleBar.svelte';
  import { settings, type AppSettings } from '$lib/stores/settings-store';
  import PageTransitions from '../components/PageTransitions.svelte';
  import { ModeWatcher } from "mode-watcher";
  import { onMount } from 'svelte';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import type { Event as TauriEvent } from '@tauri-apps/api/event';

  let isMaximized = false;
  let isResizing = false;

  onMount(async () => {
    const window = await getCurrentWindow();
    isMaximized = await window.isMaximized();

    window.listen('tauri://resize', () => {
      isResizing = true;
      setTimeout(() => {
        isResizing = false;
      }, 600);
    });

    window.listen('tauri://maximize', async () => {
      isMaximized = await window.isMaximized();
    });

    window.listen('tauri://unmaximize', () => {
      isMaximized = false;
    });

    // Listen for titlebar setting changes
    await window.listen('titlebar-settings-changed', (event: TauriEvent<AppSettings>) => {
        if (event.payload) {
            settings.set(event.payload);
        }
    });
  });
</script>

<ModeWatcher defaultMode="dark" />
<div 
  class="window-container"
  class:maximized={isMaximized}
  class:resizing={isResizing}
>
  <div class="window">
    {#if $settings.useCustomTitlebar}
      <TitleBar 
        title="XIVloader" 
        centerTitle={$settings.centerTitle}
        showMinimize={$settings.showMinimize}
        showMaximize={$settings.showMaximize}
      />
      <div class="window-content">
        <PageTransitions>
          <slot />
        </PageTransitions>
      </div>
    {:else}
      <PageTransitions>
        <slot />
      </PageTransitions>
    {/if}
  </div>
</div>

<Toaster richColors closeButton position="top-right" />

<style>
  :global(body) {
    margin: 0;
    padding: 0;
    background: transparent;
    height: 100vh;
    overflow: hidden;
  }

  :global(html) {
    background: transparent;
  }

  .window-container {
    width: 100vw;
    height: 100vh;
    padding: 10px;
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
  }

  .window-container.maximized {
    padding: 0;
  }

  .window-container.resizing .window {
    transition: none;
  }

  .window {
    width: 100%;
    height: 100%;
    background-color: var(--background);
    border-radius: 12px;
    overflow: hidden;
    display: flex;
    flex-direction: column;
    border: 1px solid var(--border);
    box-shadow: 0 2px 12px rgba(0, 0, 0, 0.15);
    transition: border-radius 200ms ease;
  }

  .maximized .window {
    border-radius: 0;
    border: none;
    box-shadow: none;
  }

  .window-content {
    flex: 1;
    overflow: auto;
    padding-top: 47px;
  }

  .app-icon {
    width: 16px;
    height: 16px;
    opacity: 0.8;
  }

  @media (prefers-color-scheme: dark) {
    .window {
      box-shadow: 0 2px 12px rgba(0, 0, 0, 0.35);
    }
  }
</style>
