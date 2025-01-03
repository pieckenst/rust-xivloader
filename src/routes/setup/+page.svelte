<script lang="ts">
    import { Button } from "$lib/components/ui/button";
    import * as Card from "$lib/components/ui/card";
    import * as Accordion from "$lib/components/ui/accordion";
    import { Input } from "$lib/components/ui/input";
    import { Label } from "$lib/components/ui/label";
    import { goto } from "$app/navigation";
    import { invoke } from "@tauri-apps/api/core";
    import { gameConfig } from '$lib/stores/game-config';
  
    let gamePath = $gameConfig.gamePath;
    let isSteam = $gameConfig.isSteam;
    let statusString = "Ready to launch";
    let logOutput: string[] = [];

    async function handleLaunch() {
      try {
        statusString = "Launching game...";
        $gameConfig.gamePath = gamePath;
        $gameConfig.isSteam = isSteam;
        
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
            region: $gameConfig.region
          }
        });
        
        statusString = "Game launched successfully";
        logOutput = [...logOutput, `${new Date().toISOString()}: ${result}`];
      } catch (error) {
        statusString = `Launch failed: ${error}`;
        logOutput = [...logOutput, `${new Date().toISOString()}: ERROR - ${error}`];
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
  
      <Card.Content class="flex max-h-full flex-grow overflow-hidden pt-2">
        <div class="relative flex flex-grow flex-col">
          <div class="p-6 flex flex-grow flex-col items-center justify-center gap-6">
            <div class="w-full space-y-4">
              <div class="space-y-2">
                <Label for="gamePath">Game Installation Path</Label>
                <Input 
                  id="gamePath" 
                  bind:value={gamePath} 
                  placeholder="Path to FFXIV installation"
                />
              </div>
  
              <div class="flex items-center space-x-2">
                <input
                  type="checkbox"
                  id="steam"
                  bind:checked={isSteam}
                  class="h-4 w-4 rounded border-input"
                />
                <Label for="steam">Launch through Steam</Label>
              </div>
  
              <div class="flex flex-col h-full">
                <!-- Inside Card.Content before buttons -->
              <Accordion.Root class="w-full mb-4">
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
              
                <div class="text-sm text-muted-foreground flex items-center gap-2 mb-16">
                  <span>Status: {statusString}</span>
                </div>
              </div>
              
              



            </div>
          </div>
  
          <div class="p-6 pt-0 absolute bottom-0 right-0 flex w-full flex-row items-center justify-between gap-2">
            <Button variant="outline" on:click={handleBack}>
              Back
            </Button>
  
            <div class="flex flex-row items-center gap-2">
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
  