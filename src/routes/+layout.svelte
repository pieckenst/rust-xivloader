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

    // Update body class based on titlebar setting
    const updateBodyClass = () => {
      if ($settings.useCustomTitlebar) {
        document.body.classList.add('custom-titlebar-enabled');
        document.documentElement.classList.add('custom-titlebar-enabled');
      } else {
        document.body.classList.remove('custom-titlebar-enabled');
        document.documentElement.classList.remove('custom-titlebar-enabled');
      }
    };

    // Initial setup
    updateBodyClass();

    // Watch for changes
    settings.subscribe(() => {
      updateBodyClass();
    });
  });
</script>

<ModeWatcher defaultMode="dark" />
<div 
  class="window-container"
  class:maximized={isMaximized}
  class:resizing={isResizing}
  class:custom-titlebar={$settings.useCustomTitlebar}
>
  {#if $settings.useCustomTitlebar}
    <div class="window-background"></div>
  {/if}
  <div class="window" class:custom-titlebar={$settings.useCustomTitlebar}>
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
    height: 100vh;
    overflow: hidden;
  }

  :global(html.custom-titlebar-enabled),
  :global(body.custom-titlebar-enabled) {
    background: transparent !important;
  }

  :global(html.custom-titlebar-enabled.maximized),
  :global(body.custom-titlebar-enabled.maximized) {
    background: hsl(var(--background)) !important;
  }

  .window-container {
    width: calc(100vw + 20px);
    height: calc(100vh + 10px);
    box-sizing: border-box;
    display: flex;
    align-items: center;
    justify-content: center;
    position: relative;
    background-color: transparent;
    margin-top: -6px;
    margin-left: -10px;
  }

  .window-container.custom-titlebar {
    padding: var(--window-padding, 10px);
    padding-top: 4px;
    background: transparent;
  }

  .window-container.maximized {
    width: 100vw !important;
    height: 100vh !important;
    margin: 0 !important;
    padding: 0 !important;
    position: fixed !important;
    inset: 0 !important;
    background-color: hsl(var(--background)) !important;
  }

  .window-container.resizing .window {
    transition: none;
  }

  .window-background {
    position: absolute;
    inset: 4px var(--window-padding, 10px) var(--window-padding, 10px) var(--window-padding, 10px);
    border-radius: var(--window-border-radius, 12px);
    background: transparent;
    pointer-events: none;
    z-index: 0;
  }

  .maximized .window-background {
    position: fixed;
    inset: 0;
    border-radius: 0;
    background-color: hsl(var(--background));
  }

  .window {
    width: 100%;
    height: 100%;
    background-color: hsl(var(--background));
    overflow: hidden;
    display: flex;
    flex-direction: column;
    position: relative;
    z-index: 1;
  }

  .window.custom-titlebar {
    border-radius: var(--window-border-radius, 12px);
    box-shadow: 0 0 0 1px hsla(var(--border) / 0.1),
                0 8px 24px hsla(0, 0%, 0%, 0.12), 
                0 2px 8px hsla(0, 0%, 0%, 0.08);
    transition: all 200ms cubic-bezier(0.4, 0, 0.2, 1);
    backdrop-filter: blur(20px);
    -webkit-backdrop-filter: blur(20px);
  }

  .maximized .window.custom-titlebar {
    position: fixed;
    inset: 0;
    width: 100vw;
    height: 100vh;
    border-radius: 0;
    box-shadow: none;
    backdrop-filter: none;
    -webkit-backdrop-filter: none;
  }

  .window-content {
    flex: 1;
    overflow: auto;
    padding-top: 47px;
    border-bottom-left-radius: inherit;
    border-bottom-right-radius: inherit;
    background-color: inherit;
  }

  .maximized .window-content {
    border-radius: 0;
    margin: 0;
    padding-top: 47px;
    width: 100vw;
    height: calc(100vh - 47px);
    position: fixed;
    top: 47px;
    left: 0;
    right: 0;
    bottom: 0;
  }

  .app-icon {
    width: 16px;
    height: 16px;
    opacity: 0.8;
  }

  @media (prefers-color-scheme: dark) {
    .window.custom-titlebar {
      box-shadow: 0 0 0 1px hsla(var(--border) / 0.2),
                  0 8px 32px hsla(0, 0%, 0%, 0.36),
                  0 2px 16px hsla(0, 0%, 0%, 0.24);
    }
  }
</style>
