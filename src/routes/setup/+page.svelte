<script lang="ts">
    import { Button, buttonVariants } from "$lib/components/ui/button/index.js";
    import * as Card from "$lib/components/ui/card/index.js";
    import * as Accordion from "$lib/components/ui/accordion/index.js";
    import { Input } from "$lib/components/ui/input/index.js";
    import { Label } from "$lib/components/ui/label/index.js";
    import { Switch } from "$lib/components/ui/switch/index.js";
    import * as Sidebar from "$lib/components/ui/sidebar/index.js";
    import TitleBar from "$lib/components/TitleBar.svelte";
    import { goto } from "$app/navigation";
    import { invoke } from "@tauri-apps/api/core";
    import { appLocalDataDir } from "@tauri-apps/api/path";
    import { gameConfig } from '$lib/stores/game-config';
    import { logStore, type LogEntry } from '$lib/stores/log-store';
    import { Home, Settings, Download, FileText, Globe, Gamepad2, Wrench, Terminal, Languages, ScrollText, Palette } from "lucide-svelte";
    import { settings, saveSettings } from '$lib/stores/settings-store';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { onMount } from 'svelte';
    import { writable } from 'svelte/store';
  
    let gamePath = $gameConfig.gamePath;
    let isSteam = $gameConfig.isSteam;
    let statusString = "Ready to launch";
    let autoScroll = true;
    let logContainer: HTMLElement;

    // Add Dalamud configuration
    let dalamudEnabled = false;
    let dalamudPath = ""; // Default path
    let dalamudInjectDelay = 0;
    let showAdvancedDalamud = false;
    let dalamudConfigPath = "";
    let dalamudPluginPath = "";
    let dalamudDevPluginPath = "";
    let dalamudAssetPath = "";

    const sidebarItems = [
      {
        name: "Game Settings",
        icon: Gamepad2,
        id: "game-settings"
      },
      {
        name: "Dalamud",
        icon: Settings,
        id: "dalamud"
      },
      {
        name: "Appearance",
        icon: Palette,
        id: "appearance"
      },
      {
        name: "Language",
        icon: Languages,
        id: "language"
      },
      {
        name: "Logs",
        icon: ScrollText,
        id: "logs"
      }
    ];

    let activeSection = "game-settings";

    // Initialize default paths when enabling Dalamud
    async function initializeDalamudPaths() {
        try {
            const localAppData = await appLocalDataDir();
            // Remove the trailing slash and add XIVLauncher directory
            const xivlauncherDir = `${localAppData.slice(0, -1)}\\XIVLauncher`;
            dalamudPath = xivlauncherDir;
            logStore.addLog(`Set Dalamud path to: ${xivlauncherDir}`);
        } catch (error) {
            logStore.addLog(`Failed to get AppData path: ${error}`);
        }
    }

    function formatDisplayLog(entry: LogEntry): string {
        const icon = entry.type === 'error' ? '❌' :
                     entry.type === 'success' ? '✅' :
                     entry.type === 'start' ? '📝' : 'ℹ️';
        return `[${entry.timestamp}] ${icon} ${entry.message}`;
    }

    async function handleLaunch() {
        try {
            statusString = "Launching game...";
            $gameConfig.gamePath = gamePath;
            $gameConfig.isSteam = isSteam;
            
            logStore.addLog("Starting game launch process...");
            
            const config = {
                game_path: $gameConfig.gamePath,
                username: $gameConfig.username,
                password: $gameConfig.password,
                otp: $gameConfig.otp || "",
                language: $gameConfig.language,
                dx11: $gameConfig.dx11,
                expansion_level: $gameConfig.expansionLevel,
                is_steam: $gameConfig.isSteam,
                region: $gameConfig.region,
                enable_dalamud: dalamudEnabled,
                dalamud_path: dalamudPath || "",
                injection_delay: dalamudInjectDelay,
                additional_launch_args: "",
                dpi_awareness: "Aware"
            };

            logStore.addLog("Sending launch command with configuration");
            const result = await invoke('launch_game', { config }) as string;
            
            // Split the result into individual lines and add each to the log
            const lines = result.split('\n');
            for (const line of lines) {
                if (line.trim()) {
                    logStore.addLog(line.trim());
                }
            }
            
            statusString = "Game launched successfully";
            logStore.addLog("Launch process completed successfully");
        } catch (error: unknown) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            // For errors, also split into lines as they may contain multiple log entries
            const errorLines = errorMessage.split('\n');
            for (const line of errorLines) {
                if (line.trim()) {
                    logStore.addLog(`ERROR: ${line.trim()}`);
                }
            }
            statusString = `Launch failed: ${errorMessage}`;
        }
    }
  
    function handleBack() {
        logStore.addLog("Navigating back to login page");
        goto("/login");
    }

    async function toggleCustomTitlebar(checked: boolean) {
        const window = await getCurrentWindow();
        try {
            if (checked) {
                // Enable custom titlebar
                await window.setDecorations(false);
                document.body.classList.add('titlebar-enabled');
            } else {
                // Disable custom titlebar
                await window.setDecorations(true);
                document.body.classList.remove('titlebar-enabled');
            }
            await saveSettings({
                ...$settings,
                useCustomTitlebar: checked
            });
        } catch (error) {
            console.error('Failed to toggle window decorations:', error);
            logStore.addLog(`Failed to toggle window decorations: ${error}`);
        }
    }

    // Initialize window decorations based on settings
    onMount(async () => {
        const window = await getCurrentWindow();
        try {
            if ($settings.useCustomTitlebar) {
                await window.setDecorations(false);
                document.body.classList.add('titlebar-enabled');
            } else {
                await window.setDecorations(true);
                document.body.classList.remove('titlebar-enabled');
            }
        } catch (error) {
            console.error('Failed to initialize window decorations:', error);
            logStore.addLog(`Failed to initialize window decorations: ${error}`);
        }
    });

    // Initialize preview settings store
    const previewSettings = writable({
        centerTitle: $settings.centerTitle,
        showMinimize: $settings.showMinimize,
        showMaximize: $settings.showMaximize
    });

    // Update preview settings when main settings change
    $: {
        if ($settings.useCustomTitlebar) {
            previewSettings.set({
                centerTitle: $settings.centerTitle,
                showMinimize: $settings.showMinimize,
                showMaximize: $settings.showMaximize
            });
        }
    }

    // Function to apply preview settings
    async function applyTitlebarSettings() {
        const window = await getCurrentWindow();
        try {
            const newSettings = {
                ...$settings,
                centerTitle: $previewSettings.centerTitle,
                showMinimize: $previewSettings.showMinimize,
                showMaximize: $previewSettings.showMaximize
            };
            
            // First update the settings store
            await saveSettings(newSettings);
            
            // Dispatch a custom event to notify layout about titlebar changes
            await window.emit('titlebar-settings-changed', newSettings);
            
            logStore.addLog("Titlebar settings applied successfully");
        } catch (error) {
            console.error('Failed to apply titlebar settings:', error);
            logStore.addLog(`Failed to apply titlebar settings: ${error}`);
        }
    }
</script>

<div class="container flex min-h-[calc(100vh-4rem)] items-center justify-center gap-4 py-6">
  <Card.Root class="w-full max-w-[1200px] flex flex-col">
    <Card.Header class="pb-0 flex flex-row items-center justify-between">
      <div class="flex items-center gap-2">
        <div class="flex h-10 w-10 items-center justify-center rounded-lg bg-primary">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-6 w-6 text-primary-foreground">
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
          </svg>
        </div>
        <div>
          <Card.Title class="text-2xl">XIV Loader</Card.Title>
          <Card.Description>Configure your game installation settings.</Card.Description>
        </div>
      </div>
    </Card.Header>

    <Card.Content class="flex flex-grow overflow-hidden pt-6">
      <div class="flex h-[600px] w-full">
        <!-- Sidebar -->
        <div class="hidden border-r md:block w-[240px] flex-shrink-0">
          <nav class="grid gap-1 p-2">
            {#each sidebarItems as item}
              <button 
                class="flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors hover:bg-muted
                       {activeSection === item.id ? 'bg-muted' : ''}"
                on:click={() => activeSection = item.id}
              >
                <svelte:component this={item.icon} class="h-4 w-4" />
                {item.name}
              </button>
            {/each}
          </nav>
        </div>

        <!-- Main Content -->
        <main class="flex-1 flex flex-col overflow-hidden">
          <header class="flex h-16 shrink-0 items-center border-b px-6">
            <h2 class="text-lg font-semibold">
              {sidebarItems.find(item => item.id === activeSection)?.name}
            </h2>
          </header>

          <div class="flex-1 overflow-y-auto p-6">
            {#if activeSection === 'game-settings'}
              <div class="space-y-6">
                <div class="space-y-2">
                  <Label for="gamePath">Game Installation Path</Label>
                  <Input 
                    id="gamePath" 
                    bind:value={gamePath} 
                    placeholder="Path to FFXIV installation"
                    class="w-full"
                  />
                </div>

                <div class="flex items-center space-x-2">
                  <Switch
                    id="steam"
                    checked={isSteam}
                    onCheckedChange={(checked) => isSteam = checked}
                  />
                  <Label for="steam">Launch through Steam</Label>
                </div>
              </div>
            {:else if activeSection === 'dalamud'}
              <div class="space-y-6">
                <div class="flex items-center justify-between">
                  <div class="space-y-0.5">
                    <Label for="dalamud">Dalamud Support</Label>
                    <div class="text-sm text-muted-foreground">
                      Enable in-game modifications and plugins
                    </div>
                  </div>
                  <Switch 
                    id="dalamud"
                    checked={dalamudEnabled}
                    onCheckedChange={async (checked) => {
                      dalamudEnabled = checked;
                      if (checked) {
                        await initializeDalamudPaths();
                      }
                    }}
                  />
                </div>

                {#if dalamudEnabled}
                  <div class="space-y-6">
                    <div class="space-y-2">
                      <Label for="dalamudPath">Dalamud Installation Path</Label>
                      <Input 
                        id="dalamudPath" 
                        bind:value={dalamudPath} 
                        placeholder="Path to Dalamud installation"
                        class="w-full"
                      />
                    </div>

                    <div class="space-y-2">
                      <Label for="injectDelay">Injection Delay (ms)</Label>
                      <Input 
                        id="injectDelay" 
                        type="number" 
                        bind:value={dalamudInjectDelay} 
                        min="0"
                        class="w-full"
                      />
                    </div>

                    <button 
                      class={buttonVariants({ variant: "outline", class: "w-full" })}
                      on:click={() => showAdvancedDalamud = !showAdvancedDalamud}
                    >
                      {showAdvancedDalamud ? 'Hide' : 'Show'} Advanced Settings
                    </button>

                    {#if showAdvancedDalamud}
                      <div class="space-y-6">
                        <div class="space-y-2">
                          <Label for="configPath">Configuration Path</Label>
                          <Input 
                            id="configPath" 
                            bind:value={dalamudConfigPath} 
                            placeholder="Custom configuration path"
                            disabled={!showAdvancedDalamud}
                            class="w-full"
                          />
                        </div>

                        <div class="space-y-2">
                          <Label for="pluginPath">Plugin Path</Label>
                          <Input 
                            id="pluginPath" 
                            bind:value={dalamudPluginPath} 
                            placeholder="Custom plugin path"
                            disabled={!showAdvancedDalamud}
                            class="w-full"
                          />
                        </div>

                        <div class="space-y-2">
                          <Label for="devPluginPath">Dev Plugin Path</Label>
                          <Input 
                            id="devPluginPath" 
                            bind:value={dalamudDevPluginPath} 
                            placeholder="Custom dev plugin path"
                            disabled={!showAdvancedDalamud}
                            class="w-full"
                          />
                        </div>

                        <div class="space-y-2">
                          <Label for="assetPath">Asset Path</Label>
                          <Input 
                            id="assetPath" 
                            bind:value={dalamudAssetPath} 
                            placeholder="Custom asset path"
                            disabled={!showAdvancedDalamud}
                            class="w-full"
                          />
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
            {:else if activeSection === 'appearance'}
              <div class="space-y-6">
                <div class="flex items-center justify-between">
                  <div class="space-y-0.5">
                    <Label>Custom Titlebar</Label>
                    <div class="text-sm text-muted-foreground">
                      Use a custom titlebar with window controls
                    </div>
                  </div>
                  <Switch 
                    id="titlebar" 
                    checked={$settings.useCustomTitlebar} 
                    onCheckedChange={toggleCustomTitlebar}
                  />
                </div>

                {#if $settings.useCustomTitlebar}
                  <div class="space-y-4">
                    <div class="flex items-center justify-between">
                      <div class="space-y-0.5">
                        <Label>Center Window Title</Label>
                        <div class="text-sm text-muted-foreground">
                          Center the window title in the titlebar
                        </div>
                      </div>
                      <Switch 
                        id="centerTitle" 
                        checked={$previewSettings.centerTitle} 
                        onCheckedChange={(checked) => previewSettings.update(s => ({ ...s, centerTitle: checked }))}
                      />
                    </div>

                    <div class="flex items-center justify-between">
                      <div class="space-y-0.5">
                        <Label>Show Minimize Button</Label>
                        <div class="text-sm text-muted-foreground">
                          Show the minimize window button
                        </div>
                      </div>
                      <Switch 
                        id="showMinimize" 
                        checked={$previewSettings.showMinimize} 
                        onCheckedChange={(checked) => previewSettings.update(s => ({ ...s, showMinimize: checked }))}
                      />
                    </div>

                    <div class="flex items-center justify-between">
                      <div class="space-y-0.5">
                        <Label>Show Maximize Button</Label>
                        <div class="text-sm text-muted-foreground">
                          Show the maximize window button
                        </div>
                      </div>
                      <Switch 
                        id="showMaximize" 
                        checked={$previewSettings.showMaximize} 
                        onCheckedChange={(checked) => previewSettings.update(s => ({ ...s, showMaximize: checked }))}
                      />
                    </div>

                    <div class="rounded-lg border p-4 space-y-4">
                      <div class="space-y-1">
                        <Label>Preview</Label>
                        <div class="text-sm text-muted-foreground">
                          Preview your titlebar settings before applying them
                        </div>
                      </div>
                      <div class="relative h-[47px] rounded-lg border bg-background overflow-hidden">
                        <div class="preview-only">
                          <TitleBar
                            title="Preview - XIVloader"
                            centerTitle={$previewSettings.centerTitle}
                            showMinimize={$previewSettings.showMinimize}
                            showMaximize={$previewSettings.showMaximize}
                          />
                        </div>
                      </div>
                      <button class={buttonVariants({ variant: "outline" })}
                        on:click={applyTitlebarSettings}
                      >Apply Titlebar Settings</button>
                        
                      
                    </div>
                  </div>
                {/if}
              </div>
            {:else if activeSection === 'language'}
              <div class="space-y-6">
                <p class="text-muted-foreground">Language settings coming soon...</p>
              </div>
            {:else if activeSection === 'logs'}
              <div class="space-y-4">
                <div class="flex items-center justify-between">
                  <span class="text-sm font-medium">Launch Log ({$logStore.length} entries)</span>
                  <div class="flex items-center gap-2">
                    <label class="flex items-center gap-1 text-xs">
                      <input type="checkbox" bind:checked={autoScroll} class="h-3 w-3">
                      Auto-scroll
                    </label>
                  </div>
                </div>

                <div bind:this={logContainer} 
                     class="space-y-1 h-[400px] overflow-y-auto font-mono text-sm rounded-lg border p-4">
                  {#if $logStore.length === 0}
                    <div class="text-sm text-muted-foreground italic text-center py-2">
                      No logs available
                    </div>
                  {:else}
                    {#each $logStore as log}
                      <div class="py-1 px-2 rounded-lg border border-muted-foreground/20 
                                {log.type === 'error' ? 'bg-red-500/10 border-red-500/20' : 
                                 log.type === 'success' ? 'bg-green-500/10 border-green-500/20' :
                                 log.type === 'start' ? 'bg-blue-500/10 border-blue-500/20' :
                                 'bg-muted/30'}">
                        {formatDisplayLog(log)}
                      </div>
                    {/each}
                  {/if}
                </div>

                <div class="flex justify-end gap-2">
                  <button 
                    class={buttonVariants({ variant: "outline", size: "sm" })}
                    on:click={() => logStore.clear()}>
                    Clear Logs
                  </button>
                  <button 
                    class={buttonVariants({ variant: "outline", size: "sm" })}
                    on:click={() => {
                      if (logContainer) {
                        logContainer.scrollTop = logContainer.scrollHeight;
                      }
                    }}>
                    Scroll to Bottom
                  </button>
                </div>
              </div>
            {/if}
          </div>

          <!-- Status Bar -->
          <div class="border-t p-4 flex justify-between items-center bg-background">
            <div class="text-sm text-muted-foreground">
              Status: {statusString}
            </div>

            <div class="flex items-center gap-2">
              <a href="/login" class={buttonVariants({ variant: "outline" })} on:click|preventDefault={handleBack}>
                Back
              </a>

              <button class={buttonVariants({ variant: "outline" })} disabled>
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2 h-4 w-4">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                  <polyline points="7 10 12 15 17 10"/>
                  <line x1="12" x2="12" y1="15" y2="3"/>
                </svg>
                Desktop
              </button>

              <button class={buttonVariants({ variant: "outline" })} on:click={handleLaunch}>
                Launch Game
              </button>
            </div>
          </div>
        </main>
      </div>
    </Card.Content>
  </Card.Root>
</div>

<style>
  /* Custom scrollbar styling */
  :global(.overflow-y-auto) {
    scrollbar-width: thin;
    scrollbar-color: rgba(155, 155, 155, 0.5) transparent;
  }

  :global(.overflow-y-auto::-webkit-scrollbar) {
    width: 6px;
  }

  :global(.overflow-y-auto::-webkit-scrollbar-track) {
    background: transparent;
  }

  :global(.overflow-y-auto::-webkit-scrollbar-thumb) {
    background-color: rgba(155, 155, 155, 0.5);
    border-radius: 20px;
    border: transparent;
  }

  /* Card size transitions */
  :global(.card) {
    transition: all 0.3s cubic-bezier(0.4, 0, 0.2, 1);
    transform-origin: center;
    will-change: transform, opacity, max-height, min-height, width;
  }

  :global(.card:focus-within) {
    transform: translateY(-2px);
    box-shadow: var(--shadow-lg);
  }

  /* Form transitions */
  .form-section {
    animation: slideUp 0.3s ease-out;
  }

  .form-section:nth-child(2) {
    animation-delay: 0.1s;
  }

  .form-section:nth-child(3) {
    animation-delay: 0.2s;
  }

  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  /* Input focus animations */
  :global(input:focus), :global(select:focus) {
    transition: all 0.2s ease;
    transform: translateY(-1px);
  }

  /* Add to existing styles */
  :global(.preview-only .titlebar) {
    position: relative !important;
    top: unset !important;
    left: unset !important;
    right: unset !important;
    z-index: 1 !important;
  }
</style>