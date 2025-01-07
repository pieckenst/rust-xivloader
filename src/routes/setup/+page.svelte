<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import * as Card from "$lib/components/ui/card";
    import * as Accordion from "$lib/components/ui/accordion";
    import { Input } from "$lib/components/ui/input";
    import { Label } from "$lib/components/ui/label";
    import { Switch } from "$lib/components/ui/switch";
    import { goto } from "$app/navigation";
    import { invoke } from "@tauri-apps/api/core";
    import { gameConfig } from '$lib/stores/game-config';
  
    let gamePath = $gameConfig.gamePath;
    let isSteam = $gameConfig.isSteam;
    let statusString = "Ready to launch";
    let logOutput: string[] = [];

    // Add Dalamud configuration
    let dalamudEnabled = false;
    let dalamudPath = ""; // Default path
    let dalamudInjectDelay = 0;
    let showAdvancedDalamud = false;
    let dalamudConfigPath = "";
    let dalamudPluginPath = "";
    let dalamudDevPluginPath = "";
    let dalamudAssetPath = "";

    async function handleLaunch() {
      try {
        statusString = "Launching game...";
        $gameConfig.gamePath = gamePath;
        $gameConfig.isSteam = isSteam;
        
        // Clear previous log output when starting a new launch
        logOutput = [];
        
        const result = await invoke('launch_game', { 
          config: {
            game_path: $gameConfig.gamePath,
            username: $gameConfig.username,
            password: $gameConfig.password,
            otp: $gameConfig.otp,
            language: $gameConfig.language,
            dx11: $gameConfig.dx11,
            expansion_level: $gameConfig.expansionLevel,
            is_steam: $gameConfig.isSteam,
            region: $gameConfig.region,
            // Add Dalamud configuration
            dalamud_enabled: dalamudEnabled,
            dalamud_path: dalamudPath || null,
            dalamud_inject_delay: dalamudInjectDelay,
            dalamud_configuration_path: dalamudConfigPath || null,
            dalamud_plugin_path: dalamudPluginPath || null,
            dalamud_dev_plugin_path: dalamudDevPluginPath || null,
            dalamud_asset_path: dalamudAssetPath || null
          }
        }) as string;
        
        // Split the result into individual lines and add each to the log
        const lines = result.split('\n');
        for (const line of lines) {
          if (line.trim()) {
            logOutput = [...logOutput, `${new Date().toISOString()}: ${line.trim()}`];
          }
        }
        
        statusString = "Game launched successfully";
      } catch (error: unknown) {
        // For errors, also split into lines as they may contain multiple log entries
        const errorLines = (error as Error).toString().split('\n');
        for (const line of errorLines) {
          if (line.trim()) {
            logOutput = [...logOutput, `${new Date().toISOString()}: ERROR - ${line.trim()}`];
          }
        }
        statusString = `Launch failed: ${error}`;
      }
    }
  
    function handleBack() {
      goto("/login");
    }
  </script>
  
  <div class="flex h-screen w-screen flex-col items-center justify-center gap-2">
    <Card.Root class="max-h-[800px] min-h-[480px] w-[800px] flex flex-col">
      <Card.Header class="pb-0 flex flex-row items-center justify-between">
        <div>
          <Card.Title class="flex flex-row items-center gap-2">
            XIV Loader
            <span class="inline-flex items-center rounded-full border px-2.5 py-0.5 text-xs font-semibold">
              1.0.0
            </span>
          </Card.Title>
          <Card.Description>
            Configure your game installation settings.
          </Card.Description>
        </div>
      </Card.Header>
  
      <Card.Content class="flex flex-grow overflow-hidden pt-2">
        <!-- Main content wrapper with proper scrolling -->
        <div class="relative flex flex-col w-full h-full">
          <!-- Scrollable content area -->
          <div class="flex-grow overflow-y-auto px-6 pb-24">
            <!-- Main content with proper spacing -->
            <div class="space-y-6 py-6">
              <!-- Game Path Section -->
              <div class="space-y-2">
                <Label for="gamePath">Game Installation Path</Label>
                <Input 
                  id="gamePath" 
                  bind:value={gamePath} 
                  placeholder="Path to FFXIV installation"
                />
              </div>
  
              <!-- Steam Option -->
              <div class="flex items-center space-x-2">
                <input
                  type="checkbox"
                  id="steam"
                  bind:checked={isSteam}
                  class="h-4 w-4 rounded border-input"
                />
                <Label for="steam">Launch through Steam</Label>
              </div>
  
              <!-- Dalamud Settings -->
              <div class="space-y-4 border rounded-lg p-4">
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
                    onCheckedChange={(checked) => dalamudEnabled = checked}
                  />
                </div>
  
                {#if dalamudEnabled}
                  <div class="space-y-4">
                    <div class="space-y-2">
                      <Label for="dalamudPath">Dalamud Installation Path</Label>
                      <Input 
                        id="dalamudPath" 
                        bind:value={dalamudPath} 
                        placeholder="Path to Dalamud installation"
                      />
                    </div>
  
                    <div class="space-y-2">
                      <Label for="injectDelay">Injection Delay (ms)</Label>
                      <Input 
                        id="injectDelay" 
                        type="number" 
                        bind:value={dalamudInjectDelay} 
                        min="0"
                      />
                    </div>
  
                    <Button 
                      variant="outline" 
                      class="w-full"
                      on:click={() => showAdvancedDalamud = !showAdvancedDalamud}
                    >
                      {showAdvancedDalamud ? 'Hide' : 'Show'} Advanced Settings
                    </Button>
  
                    {#if showAdvancedDalamud}
                      <div class="space-y-4">
                        <div class="space-y-2">
                          <Label for="configPath">Configuration Path</Label>
                          <Input 
                            id="configPath" 
                            bind:value={dalamudConfigPath} 
                            placeholder="Custom configuration path"
                          />
                        </div>
  
                        <div class="space-y-2">
                          <Label for="pluginPath">Plugin Path</Label>
                          <Input 
                            id="pluginPath" 
                            bind:value={dalamudPluginPath} 
                            placeholder="Custom plugin path"
                          />
                        </div>
  
                        <div class="space-y-2">
                          <Label for="devPluginPath">Dev Plugin Path</Label>
                          <Input 
                            id="devPluginPath" 
                            bind:value={dalamudDevPluginPath} 
                            placeholder="Custom dev plugin path"
                          />
                        </div>
  
                        <div class="space-y-2">
                          <Label for="assetPath">Asset Path</Label>
                          <Input 
                            id="assetPath" 
                            bind:value={dalamudAssetPath} 
                            placeholder="Custom asset path"
                          />
                        </div>
                      </div>
                    {/if}
                  </div>
                {/if}
              </div>
  
              <!-- Launch Log Section -->
              <Accordion.Root class="w-full">
                <Accordion.Item value="launch-log">
                  <Accordion.Trigger class="flex w-full items-center justify-between px-4 py-2 text-sm font-medium hover:bg-muted/50 transition-colors">
                    Launch Log ({logOutput.length} entries)
                  </Accordion.Trigger>
                  <Accordion.Content class="px-4 py-2">
                    <div class="space-y-2 max-h-[200px] overflow-y-auto">
                      {#if logOutput.length === 0}
                        <div class="text-sm text-muted-foreground italic text-center py-2">
                          No logs available
                        </div>
                      {:else}
                        {#each logOutput as log}
                          <div class="text-sm font-mono bg-muted/30 p-2 rounded border border-muted-foreground/20">
                            {log}
                          </div>
                        {/each}
                      {/if}
                    </div>
                    <div class="mt-4 flex justify-end">
                      <Button 
                        variant="outline" 
                        size="sm"
                        on:click={() => logOutput = []}
                      >
                        Clear Logs
                      </Button>
                    </div>
                  </Accordion.Content>
                </Accordion.Item>
              </Accordion.Root>
  
              <!-- Status Text -->
              <div class="text-sm text-muted-foreground flex items-center gap-2">
                <span>Status: {statusString}</span>
              </div>
            </div>
          </div>
  
          <!-- Fixed bottom buttons -->
          <div class="absolute bottom-0 left-0 right-0 p-6 bg-background border-t flex justify-between items-center">
            <Button variant="outline" on:click={handleBack}>
              Back
            </Button>
  
            <div class="flex items-center gap-2">
              <Button variant="outline" disabled>
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2 h-4 w-4">
                  <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                  <polyline points="7 10 12 15 17 10"/>
                  <line x1="12" x2="12" y1="15" y2="3"/>
                </svg>
                Desktop
              </Button>
  
              <Button variant="outline" on:click={handleLaunch}>
                Launch Game
              </Button>
            </div>
          </div>
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
  </style>