<script lang="ts">
  import { Button, buttonVariants } from "$lib/components/ui/button/index.js";
  import * as Card from "$lib/components/ui/card/index.js";
  import * as Accordion from "$lib/components/ui/accordion/index.js";
  import * as AlertDialog from "$lib/components/ui/alert-dialog/index.js";
  import Sun from "lucide-svelte/icons/sun";
  import Moon from "lucide-svelte/icons/moon";
  import Info from "lucide-svelte/icons/info";
  import { toggleMode } from "mode-watcher";
  import { onMount } from 'svelte';
  import { goto } from "$app/navigation";
  import { listen } from '@tauri-apps/api/event';
  import { logStore, type LogEntry } from '$lib/stores/log-store';

  let logOutput: string[] = [];
  async function handleThemeToggle(event: MouseEvent) {
    const target = event.currentTarget as HTMLElement;
    if (!document.startViewTransition || !target) return;

    const transition = document.startViewTransition(() => {
      toggleMode();
    });

    await transition.ready;

    const { top, left } = target.getBoundingClientRect();
    const right = window.innerWidth - left;
    const bottom = window.innerHeight - top;
    
    const maxRadius = Math.hypot(
      Math.max(left, right),
      Math.max(top, bottom)
    );

    document.documentElement.animate(
      {
        clipPath: [
          `circle(0px at ${left}px ${top}px)`,
          `circle(${maxRadius}px at ${left}px ${top}px)`
        ],
      },
      {
        duration: 500,
        easing: 'ease-in-out',
        pseudoElement: '::view-transition-new(root)'
      }
    );
  }

  function addLog(message: string) {
    const timestamp = new Date().toISOString();
    logOutput = [...logOutput, `[${timestamp}] ${message}`];
  }

  function formatDisplayLog(entry: LogEntry): string {
    const icon = entry.type === 'error' ? '❌' :
                 entry.type === 'success' ? '✅' :
                 entry.type === 'start' ? '📝' : 'ℹ️';
    return `[${entry.timestamp}] ${icon} ${entry.message}`;
  }

  function handleNext() {
    logStore.addLog("Navigating to login page");
    goto("/login", { replaceState: true, invalidateAll: true });
  }

  onMount(() => {
    logStore.addLog("Application started");

    let unlistenTauri: (() => void) | undefined;

    // Listen for Tauri events
    listen('tauri://event', (event) => {
      console.log('Event received:', event);
    }).then((unlisten) => {
      unlistenTauri = unlisten;
    });

    const errorHandler = (event: ErrorEvent) => {
      logStore.addLog(`Error: ${event.message}`);
    };

    const rejectionHandler = (event: PromiseRejectionEvent) => {
      logStore.addLog(`Unhandled Promise Rejection: ${event.reason}`);
    };

    window.addEventListener('error', errorHandler);
    window.addEventListener('unhandledrejection', rejectionHandler);

    return () => {
      if (unlistenTauri) {
        unlistenTauri();
      }
      window.removeEventListener('error', errorHandler);
      window.removeEventListener('unhandledrejection', rejectionHandler);
    };
  });
</script>

<div class="flex h-screen w-screen flex-col items-center justify-center gap-2">
  <Card.Root class="max-h-[800px] min-h-[480px] w-[800px] flex flex-col">
    <Card.Header class="pb-0 flex flex-row items-center justify-between">
      <div class="flex items-center gap-2">
        <div class="flex h-10 w-10 items-center justify-center rounded-lg bg-primary">
          <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-6 w-6 text-primary-foreground">
            <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
          </svg>
        </div>
        <div>
          <Card.Title class="text-2xl">XIV Loader</Card.Title>
          <Card.Description>The easiest way to launch Final Fantasy XIV.</Card.Description>
        </div>
      </div>
      <div class="flex items-center gap-2">
        <button 
          class={buttonVariants({ variant: "outline", size: "icon" })}
          on:click={handleThemeToggle}
        >
          <Sun
            class="h-[1.2rem] w-[1.2rem] rotate-0 scale-100 transition-all duration-300 dark:-rotate-90 dark:scale-0"
          />
          <Moon
            class="absolute h-[1.2rem] w-[1.2rem] rotate-90 scale-0 transition-all duration-300 dark:rotate-0 dark:scale-100"
          />
          <span class="sr-only">Toggle theme</span>
        </button>

        <a 
          href="https://github.com/pieckenst/rust-xivloader" 
          target="_blank"
          class={buttonVariants({ variant: "outline", size: "icon" })}
          aria-label="View source on GitHub"
        >
          <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" class="h-4 w-4">
            <path d="M9 19c-5 1.5-5-2.5-7-3m14 6v-3.87a3.37 3.37 0 0 0-.94-2.61c3.14-.35 6.44-1.54 6.44-7A5.44 5.44 0 0 0 20 4.77 5.07 5.07 0 0 0 19.91 1S18.73.65 16 2.48a13.38 13.38 0 0 0-7 0C6.27.65 5.09 1 5.09 1A5.07 5.07 0 0 0 5 4.77a5.44 5.44 0 0 0-1.5 3.78c0 5.42 3.3 6.61 6.44 7A3.37 3.37 0 0 0 9 18.13V22"/>
          </svg>
        </a>
      </div>
    </Card.Header>

    <Card.Content class="flex max-h-full flex-grow overflow-hidden pt-2">
      <div class="relative flex flex-grow flex-col">
        <div class="p-6 flex flex-grow flex-col items-center justify-center gap-2">
          <div class="flex flex-col items-center justify-center">
            <h1 class="text-2xl font-bold">Welcome to XIV Loader</h1>
            <h2 class="text-muted-foreground">Click <b>Next</b> to proceed with login.</h2>
          </div>
        </div>

        <div class="p-6 pt-0 absolute bottom-0 right-0 flex w-full flex-row items-center justify-between gap-2">
          <AlertDialog.Root>
            <AlertDialog.Trigger>
              <button class={buttonVariants({ variant: "outline" })}>
                <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2 h-4 w-4">
                  <circle cx="12" cy="12" r="10"/>
                  <path d="M12 16v-4"/>
                  <path d="M12 8h.01"/>
                </svg>
                Troubleshooting
              </button>
            </AlertDialog.Trigger>
          
            <AlertDialog.Content class="sm:max-w-[725px] max-h-[85vh] overflow-hidden">
              <AlertDialog.Header>
                <AlertDialog.Title class="flex flex-row items-center gap-2 text-lg font-semibold">
                  <Info class="h-5 w-5" />
                  Troubleshooting
                </AlertDialog.Title>
              </AlertDialog.Header>
          
              <div class="flex-1 overflow-y-auto px-4 py-3">
                <Accordion.Root type="multiple" class="w-full space-y-2">
                  <Accordion.Item value="game-launch">
                    <Accordion.Trigger class="w-full px-4 py-2 text-sm font-medium">
                      Game Launch Issues
                    </Accordion.Trigger>
                    <Accordion.Content class="px-4 py-2 text-sm">
                      <div class="space-y-2">
                        <p>If FFXIV fails to launch, try the following:</p>
                        <ul class="list-disc pl-4 space-y-1">
                          <li>Verify game files integrity through the launcher</li>
                          <li>Check if DirectX and Visual C++ Redistributables are installed</li>
                          <li>Ensure your graphics drivers are up to date</li>
                        </ul>
                      </div>
                    </Accordion.Content>
                  </Accordion.Item>
          
                  <Accordion.Item value="app-logs">
                    <Accordion.Trigger class="w-full px-4 py-2 text-sm font-medium">
                      Application Logs
                    </Accordion.Trigger>
                    <Accordion.Content class="px-4 py-2">
                      <div class="space-y-2 max-h-[200px] overflow-y-auto">
                        {#if $logStore.length === 0}
                          <div class="text-sm text-muted-foreground italic text-center py-2">
                            No logs available
                          </div>
                        {:else}
                          {#each $logStore as log}
                            <div class="text-sm font-mono p-2 rounded border border-muted-foreground/20
                                      {log.type === 'error' ? 'bg-red-500/10 border-red-500/20' : 
                                       log.type === 'success' ? 'bg-green-500/10 border-green-500/20' :
                                       log.type === 'start' ? 'bg-blue-500/10 border-blue-500/20' :
                                       'bg-muted/30'}">
                              {formatDisplayLog(log)}
                            </div>
                          {/each}
                        {/if}
                        <div class="mt-4 flex justify-end">
                          <button 
                            class={buttonVariants({ variant: "outline", size: "sm" })}
                            on:click={() => logStore.clear()}
                          >
                            Clear Logs
                          </button>
                        </div>
                      </div>
                    </Accordion.Content>
                  </Accordion.Item>
          
                  <Accordion.Item value="common-issues">
                    <Accordion.Trigger class="w-full px-4 py-2 text-sm font-medium">
                      Common Issues
                    </Accordion.Trigger>
                    <Accordion.Content class="px-4 py-2 text-sm">
                      <div class="space-y-2">
                        <ul class="list-disc pl-4 space-y-1">
                          <li>Login authentication errors</li>
                          <li>Game crashes on startup</li>
                          <li>Performance issues</li>
                          <li>Network connectivity problems</li>
                        </ul>
                      </div>
                    </Accordion.Content>
                  </Accordion.Item>
          
                  <Accordion.Item value="support">
                    <Accordion.Trigger class="w-full px-4 py-2 text-sm font-medium">
                      Get Support
                    </Accordion.Trigger>
                    <Accordion.Content class="px-4 py-2 text-sm">
                      <div class="space-y-2">
                        <p>Need additional help?</p>
                        <ul class="list-disc pl-4 space-y-1">
                          <li>Visit our documentation</li>
                          <li>Join our Discord community</li>
                          <li>Open a GitHub issue</li>
                        </ul>
                      </div>
                    </Accordion.Content>
                  </Accordion.Item>
                </Accordion.Root>
              </div>
          
              <AlertDialog.Footer class="sm:justify-start">
                <AlertDialog.Cancel>Close</AlertDialog.Cancel>
              </AlertDialog.Footer>
            </AlertDialog.Content>
          </AlertDialog.Root>
          
        
          <div class="flex flex-row items-center gap-2">
            <button class={buttonVariants({ variant: "outline" })} disabled>
              <svg xmlns="http://www.w3.org/2000/svg" width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="mr-2 h-4 w-4">
                <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/>
                <polyline points="7 10 12 15 17 10"/>
                <line x1="12" x2="12" y1="15" y2="3"/>
              </svg>
              Desktop
            </button>
        
            <button class={buttonVariants({ variant: "outline" })} on:click={handleNext}>
              Next
            </button>
          </div>
        </div>
        
      </div>
    </Card.Content>
  </Card.Root>
</div>

<style>
  ::view-transition-old(root),
  ::view-transition-new(root) {
    animation: none;
    mix-blend-mode: normal;
  }

  ::view-transition-old(root) {
    z-index: 1;
  }
  
  ::view-transition-new(root) {
    z-index: 2147483646;
  }
  .dark::view-transition-old(root) {
    z-index: 2147483646;
  }
  
  .dark::view-transition-new(root) {
    z-index: 1;
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

  /* Content animations */
  .content-section {
    animation: fadeScale 0.4s ease-out;
  }

  @keyframes fadeScale {
    from {
      opacity: 0;
      transform: scale(0.98);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  /* Button hover animations */
  :global(button:not(:disabled)) {
    transition: all 0.2s ease;
  }

  :global(button:not(:disabled):hover) {
    transform: translateY(-1px);
  }

  :global(button:not(:disabled):active) {
    transform: translateY(0);
  }
</style>