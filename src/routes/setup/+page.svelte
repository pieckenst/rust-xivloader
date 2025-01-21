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
    import { settings, saveSettings } from '$lib/stores/settings-store';
    import { cloudBackup, AUTO_SYNC_COOLDOWN, lastSettingsSyncStore, lastGameConfigSyncStore } from '$lib/stores/cloud-backup-store';
    import { getCurrentWindow } from '@tauri-apps/api/window';
    import { onMount } from 'svelte';
    import { writable, get } from 'svelte/store';
    import { 
        Settings, 
        Gamepad2, 
        Languages, 
        ScrollText, 
        Palette,
        Cloud 
    } from "lucide-svelte";
    import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
    import { toast } from 'svelte-sonner';

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

    let cloudEmail = '';
    let cloudPassword = '';
    let isRegistering = false;

    let pendingCredentialsSync = false;

    let showCredentialsSyncDialog = false;

    let hasInitializedCloudBackup = false;

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
        name: "Cloud Backup",
        icon: Cloud,
        id: "cloud-backup"
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
            const errorMessage = error instanceof Error ? error.message : String(error);
            logStore.addLog(`Failed to get AppData path: ${errorMessage}`);
            console.error('Failed to initialize Dalamud paths:', error);
        }
    }

    function formatDisplayLog(entry: LogEntry): string {
        try {
            const icon = entry.type === 'error' ? '❌' :
                        entry.type === 'success' ? '✅' :
                        entry.type === 'start' ? '📝' : 'ℹ️';
            return `[${entry.timestamp}] ${icon} ${entry.message}`;
        } catch (error) {
            console.error('Error formatting log entry:', error);
            return '[ERROR] Failed to format log entry';
        }
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
            console.error('Game launch error:', error);
        }
    }
  
    function handleBack() {
        try {
            logStore.addLog("Navigating back to login page");
            goto("/login");
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            logStore.addLog(`Failed to navigate back: ${errorMessage}`);
            console.error('Navigation error:', error);
        }
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
            const errorMessage = error instanceof Error ? error.message : String(error);
            console.error('Failed to toggle window decorations:', error);
            logStore.addLog(`Failed to toggle window decorations: ${errorMessage}`);
            // Revert the UI state if the operation failed
            $settings.useCustomTitlebar = !checked;
        }
    }

    // Subscribe to settings changes - but only initialize once
    $: if ($settings && !hasInitializedCloudBackup) {
        logStore.addLog('Setup page: Settings updated');
        if ($settings.cloudBackupEnabled && !$cloudBackup.initialSyncComplete) {
            hasInitializedCloudBackup = true;  // Set flag before initializing
            initializeCloudBackup();
        }
    }

    async function initializeCloudBackup() {
        try {
            logStore.addLog('Setup page: Initializing cloud backup');
            if ($settings.cloudBackupEnabled && $cloudBackup.isLoggedIn) {
                // Restore settings from cloud if this is the first sync
                if (!$cloudBackup.initialSyncComplete) {
                    logStore.addLog('Setup page: First launch with cloud backup, restoring settings');
                    const restored = await cloudBackup.restoreFromCloud();
                    if (restored) {
                        logStore.addLog('Setup page: Successfully restored settings from cloud');
                        toast.success('Settings restored from cloud backup');
                    }
                }
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            logStore.addLog(`Setup page: Failed to initialize cloud backup: ${errorMessage}`);
        }
    }

    async function initializeSetupPage() {
        try {
            logStore.addLog('[Setup] Starting initialization sequence');
            
            // First wait for local settings to fully load
            logStore.addLog('[Setup] Waiting for local settings to stabilize (20s)...');
            await new Promise(resolve => setTimeout(resolve, 20000));
            
            // Load local settings
            await initializeSettings();
            logStore.addLog('[Setup] Local settings loaded');

            // Check if cloud backup should be enabled based on local settings
            if ($settings.cloudBackupEnabled && !hasInitializedCloudBackup) {
                logStore.addLog('[Setup] Cloud backup enabled in local settings, starting cloud initialization');
                hasInitializedCloudBackup = true;
                
                // Initialize cloud backup first
                await cloudBackup.initialize();
                logStore.addLog('[Setup] Cloud backup initialized');

                // Check if we have credentials to sync
                const hasCredentials = $gameConfig.username && $gameConfig.password;
                logStore.addLog(`[Setup] Credentials check - Has credentials: ${!!hasCredentials}`);
                
                // If cloud backup is logged in, handle syncing
                if ($cloudBackup.isLoggedIn) {
                    logStore.addLog('[Setup] Cloud backup logged in, checking sync requirements');
                    
                    // If we have credentials and sync is enabled, sync them first
                    if (hasCredentials && $settings.cloudBackupCredentialsSync) {
                        logStore.addLog('[Setup] Syncing credentials first');
                        await cloudBackup.syncToCloud();
                        await new Promise(resolve => setTimeout(resolve, 2000)); // Wait 2s between operations
                    }

                    // Then sync other settings
                    logStore.addLog('[Setup] Syncing general settings');
                    await cloudBackup.syncToCloud();
                    await new Promise(resolve => setTimeout(resolve, 2000)); // Wait 2s between operations
                    
                    // After syncing, check if we need to restore
                    if (!$cloudBackup.initialSyncComplete) {
                        logStore.addLog('[Setup] First launch detected, attempting restore');
                        const restored = await cloudBackup.restoreFromCloud();
                        if (restored) {
                            logStore.addLog('[Setup] Successfully restored settings from cloud');
                            toast.success('Settings restored from cloud backup');
                        }
                    } else {
                        logStore.addLog('[Setup] Not first launch, skipping restore');
                    }
                } else {
                    logStore.addLog('[Setup] Cloud backup not logged in, skipping sync/restore');
                }
            } else {
                logStore.addLog('[Setup] Cloud backup not enabled in local settings or already initialized');
            }

            logStore.addLog('[Setup] Initialization sequence completed');
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            logStore.addLog(`[Setup] Initialization failed: ${errorMessage}`);
            if (error instanceof Error && error.stack) {
                logStore.addLog(`[Setup] Error stack: ${error.stack}`);
            }
            toast.error('Failed to initialize setup page');
        }
    }

    onMount(() => {
        logStore.addLog('Setup page: Component mounted');
        initializeSetupPage();
    });

    // Initialize window decorations based on settings
    async function initializeSettings() {
        const window = await getCurrentWindow();
        try {
            logStore.addLog('[Setup] Initializing window decorations');
            if ($settings.useCustomTitlebar) {
                await window.setDecorations(false);
                document.body.classList.add('titlebar-enabled');
            } else {
                await window.setDecorations(true);
                document.body.classList.remove('titlebar-enabled');
            }
            logStore.addLog('[Setup] Window decorations initialized');
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            console.error('[Setup] Failed to initialize:', error);
            logStore.addLog(`[Setup] Failed to initialize window decorations: ${errorMessage}`);
            try {
                await window.setDecorations(true);
                document.body.classList.remove('titlebar-enabled');
            } catch (recoveryError) {
                console.error('[Setup] Failed to recover from initialization error:', recoveryError);
            }
        }
    }

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
            await window.emit('titlebar-settings-changed', {
                useCustomTitlebar: $settings.useCustomTitlebar,
                centerTitle: $settings.centerTitle,
                showMinimize: $settings.showMinimize,
                showMaximize: $settings.showMaximize
            });
            
            logStore.addLog("Titlebar settings applied successfully");

            // Update window decorations based on settings
            if (newSettings.useCustomTitlebar) {
                logStore.addLog('Enabling custom titlebar');
                await window.setDecorations(false);
                document.documentElement.classList.add('custom-titlebar-enabled');
                document.body.classList.add('titlebar-enabled');
            } else {
                logStore.addLog('Disabling custom titlebar');
                await window.setDecorations(true);
                document.documentElement.classList.remove('custom-titlebar-enabled');
                document.body.classList.remove('titlebar-enabled');
            }

            // Update maximized state
            const isMaximized = await window.isMaximized();
            if (isMaximized) {
                document.documentElement.classList.add('maximized');
                document.body.classList.add('maximized');
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            console.error('Failed to apply titlebar settings:', error);
            logStore.addLog(`Failed to apply titlebar settings: ${errorMessage}`);
            // Revert preview settings to match current settings
            previewSettings.set({
                centerTitle: $settings.centerTitle,
                showMinimize: $settings.showMinimize,
                showMaximize: $settings.showMaximize
            });
        }
    }

    async function handleCloudLogin() {
        try {
            const success = await cloudBackup.login(cloudEmail, cloudPassword);
            if (success) {
                cloudEmail = '';
                cloudPassword = '';
                // Restore settings after successful login
                await cloudBackup.restoreFromCloud();
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            console.error('Cloud login error:', error);
            logStore.addLog(`Cloud login failed: ${errorMessage}`);
        }
    }

    async function handleCloudRegister() {
        try {
            const success = await cloudBackup.register(cloudEmail, cloudPassword);
            if (success) {
                cloudEmail = '';
                cloudPassword = '';
                isRegistering = false;
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            console.error('Cloud registration error:', error);
            logStore.addLog(`Cloud registration failed: ${errorMessage}`);
        }
    }

    async function handleCloudLogout() {
        try {
            await cloudBackup.logout();
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            console.error('Cloud logout error:', error);
            logStore.addLog(`Cloud logout failed: ${errorMessage}`);
        }
    }

    async function handleAutoSync(type: 'settings' | 'gameConfig') {
        try {
            if ($cloudBackup.autoSync && $cloudBackup.enabled) {
                const now = Date.now();
                
                // Check if we should skip based on type and cooldown
                if (type === 'settings' && now - get(lastSettingsSyncStore) < AUTO_SYNC_COOLDOWN) {
                    return; // Skip if within settings cooldown
                }
                if (type === 'gameConfig' && now - get(lastGameConfigSyncStore) < AUTO_SYNC_COOLDOWN) {
                    return; // Skip if within game config cooldown
                }
                if (type === 'gameConfig' && !$cloudBackup.syncCredentials) {
                    return; // Don't sync game config if credentials sync is disabled
                }

                await cloudBackup.syncToCloud();
                logStore.addLog(`Auto-synced ${type} to cloud`);
            }
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : String(error);
            console.error('Auto-sync error:', error);
            logStore.addLog(`Auto-sync failed: ${errorMessage}`);
        }
    }
</script>

<div class="container flex min-h-[calc(100vh-4rem)] items-center justify-center gap-4 py-6">
  <Card.Root class="w-full max-w-[1200px] flex flex-col overflow-hidden">
    <Card.Content class="flex flex-grow p-0">
      <div class="flex h-[700px] w-full">
        <!-- Sidebar -->
        <div class="dark-sidebar bg-[#242424] border-r border-[#1a1a1a] md:block w-[240px] flex-shrink-0">
          <div class="flex h-16 items-center gap-2 px-4 border-b border-[#1a1a1a]">
            <div class="flex h-8 w-8 items-center justify-center rounded-lg bg-primary">
              <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4 text-primary-foreground">
                <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
              </svg>
            </div>
            <div class="font-semibold text-white">XIV Loader</div>
          </div>
          <nav class="grid gap-1 p-2">
            {#each sidebarItems as item}
              <button 
                class="sidebar-btn flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors text-[#e6e6e6]
                       hover:bg-[#3a3a3a] {activeSection === item.id ? 'bg-[#3a3a3a]' : ''}"
                on:click={() => activeSection = item.id}
              >
                <svelte:component this={item.icon} class="h-4 w-4" />
                {item.name}
              </button>
            {/each}
          </nav>
        </div>

        <!-- Main Content -->
        <main class="flex-1 flex flex-col overflow-hidden bg-background">
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
            {:else if activeSection === 'cloud-backup'}
              <div class="space-y-6">
                {#if !$cloudBackup.isLoggedIn}
                  <div class="space-y-4">
                    <div class="flex items-center justify-between mb-4">
                      <h3 class="text-lg font-semibold">
                        {isRegistering ? 'Create Account' : 'Login to Cloud Backup'}
                      </h3>
                      <button 
                        class={buttonVariants({ variant: "ghost" })}
                        on:click={() => isRegistering = !isRegistering}
                      >
                        {isRegistering ? 'Back to Login' : 'Create Account'}
                      </button>
                    </div>

                    <div class="space-y-2">
                      <Label for="cloudEmail">Email</Label>
                      <Input 
                        id="cloudEmail" 
                        type="email"
                        bind:value={cloudEmail}
                        placeholder="Enter your email"
                      />
                    </div>

                    <div class="space-y-2">
                      <Label for="cloudPassword">Password</Label>
                      <Input 
                        id="cloudPassword" 
                        type="password"
                        bind:value={cloudPassword}
                        placeholder="Enter your password"
                      />
                    </div>

                    <button 
                      class={buttonVariants()} 
                      on:click={isRegistering ? handleCloudRegister : handleCloudLogin}
                    >
                      {isRegistering ? 'Create Account' : 'Login to Cloud Backup'}
                    </button>

                    {#if isRegistering}
                      <div class="text-sm text-muted-foreground mt-2">
                        By creating an account, you agree to store your settings in the cloud.
                        Your data will be encrypted and can be deleted at any time.
                      </div>
                    {/if}
                  </div>
                {:else}
                  <div class="space-y-4">
                    <div class="flex items-center justify-between">
                      <div class="space-y-0.5">
                        <Label>Cloud Backup</Label>
                        <div class="text-sm text-muted-foreground">
                          Backup your settings and game credentials to the cloud
                        </div>
                      </div>
                      <Switch 
                        checked={$cloudBackup.enabled}
                        onCheckedChange={(checked) => cloudBackup.toggleCloudBackup(checked)}
                      />
                    </div>

                    {#if $cloudBackup.enabled}
                      <div class="space-y-4">
                        <div class="flex items-center justify-between">
                          <div class="space-y-0.5">
                            <Label>Auto Sync</Label>
                            <div class="text-sm text-muted-foreground">
                              Automatically sync settings when changes are made
                            </div>
                          </div>
                          <Switch 
                            checked={$cloudBackup.autoSync}
                            onCheckedChange={(checked) => cloudBackup.toggleAutoSync(checked)}
                          />
                        </div>

                        <div class="flex flex-row items-center justify-between rounded-lg border p-4">
                          <div class="space-y-0.5">
                            <Label>Credentials Sync</Label>
                            <div class="text-sm text-muted-foreground">
                              Include login credentials in cloud backup (encrypted)
                            </div>
                            <div class="text-xs text-yellow-500">
                              Warning: Only enable if you trust this service
                            </div>
                          </div>
                          <div class="flex items-center gap-2">
                            <Switch 
                              checked={$cloudBackup.syncCredentials}
                              onCheckedChange={(checked) => {
                                if (checked) {
                                  showCredentialsSyncDialog = true;
                                } else {
                                  cloudBackup.toggleCredentialsSync(false);
                                }
                              }}
                            />
                          </div>
                        </div>

                        <AlertDialog.Root 
                          bind:open={showCredentialsSyncDialog}
                        >
                          <AlertDialog.Content>
                            <AlertDialog.Header>
                              <AlertDialog.Title>Enable Credentials Sync?</AlertDialog.Title>
                              <AlertDialog.Description>
                                This will store your login credentials in the cloud (encrypted).
                                Only enable this if you trust this service with your login information.
                              </AlertDialog.Description>
                            </AlertDialog.Header>
                            <AlertDialog.Footer>
                              <AlertDialog.Cancel>Cancel</AlertDialog.Cancel>
                              <AlertDialog.Action>
                                <button
                                  class={buttonVariants()}
                                  on:click={() => {
                                    showCredentialsSyncDialog = false;
                                    cloudBackup.toggleCredentialsSync(true);
                                  }}
                                >
                                  Enable Sync
                                </button>
                              </AlertDialog.Action>
                            </AlertDialog.Footer>
                          </AlertDialog.Content>
                        </AlertDialog.Root>

                        <div class="rounded-lg border p-4">
                          <div class="space-y-2">
                            <div class="text-sm">
                              Last synced: {$cloudBackup.lastSyncTimestamp ? new Date($cloudBackup.lastSyncTimestamp).toLocaleString() : 'Never'}
                            </div>
                            <div class="flex gap-2">
                              <button 
                                class={buttonVariants({ variant: "outline" })} 
                                on:click={() => cloudBackup.syncToCloud()}
                              >
                                Sync Now
                              </button>
                              <button 
                                class={buttonVariants({ variant: "outline" })} 
                                on:click={() => cloudBackup.restoreFromCloud()}
                              >
                                Restore from Cloud
                              </button>
                            </div>
                          </div>
                        </div>
                      </div>
                    {/if}

                    <div class="pt-2">
                      <button class={buttonVariants({ variant: "outline", class: "text-destructive" })} on:click={handleCloudLogout}>
                        Logout from Cloud Backup
                      </button>
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

<style lang="css">
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

  /* GNOME-style transitions for sidebar items */
  :global(.sidebar-btn) {
    transition: all 0.2s ease;
  }

  :global(.sidebar-btn:hover) {
    background-color: rgba(255, 255, 255, 0.1);
  }

  :global(.sidebar-btn:active) {
    background-color: rgba(255, 255, 255, 0.15);
  }

  /* Dark sidebar scrollbar */
  :global(.dark-sidebar .overflow-y-auto) {
    scrollbar-width: thin;
    scrollbar-color: rgba(255, 255, 255, 0.2) transparent;
  }

  :global(.dark-sidebar .overflow-y-auto::-webkit-scrollbar) {
    width: 6px;
  }

  :global(.dark-sidebar .overflow-y-auto::-webkit-scrollbar-track) {
    background: transparent;
  }

  :global(.dark-sidebar .overflow-y-auto::-webkit-scrollbar-thumb) {
    background-color: rgba(255, 255, 255, 0.2);
    border-radius: 20px;
    border: transparent;
  }
</style>