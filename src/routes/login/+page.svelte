<script lang="ts">
  import * as Card from "$lib/components/ui/card";
  import * as Tabs from "$lib/components/ui/tabs";
  import { Button } from "$lib/components/ui/button";
  import { Input } from "$lib/components/ui/input";
  import { Label } from "$lib/components/ui/label";
  import { goto } from "$app/navigation";
  import { gameConfig } from '$lib/stores/game-config';
  
  let username = '';
  let password = '';
  let otp = '';
  let activeTab = 'login';

  function handleTabChange(value: string | undefined) {
    if (!document.startViewTransition || !value) return;

    document.startViewTransition(() => {
      activeTab = value;
    });
  }

  function handleNext() {
    $gameConfig.username = username;
    $gameConfig.password = password;
    $gameConfig.otp = otp;
    goto("/setup", { replaceState: true });
  }

  function handleBack() {
    goto("/", { replaceState: true });
  }
</script>
  
  <div class="flex h-screen w-screen flex-col items-center justify-center gap-2 overflow-hidden">
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
            Enter your FFXIV account details to continue.
          </Card.Description>
        </div>
      </Card.Header>
      <Card.Content class="flex max-h-full flex-grow overflow-hidden pt-2">
        <div class="relative flex flex-grow flex-col">
          <div class="p-6 flex flex-grow flex-col items-center justify-center gap-2">
            <Tabs.Root value={activeTab} onValueChange={handleTabChange} class="w-full">
              <Tabs.List class="grid w-full grid-cols-2">
                <Tabs.Trigger value="login">Login</Tabs.Trigger>
                <Tabs.Trigger value="otp">One-Time Password</Tabs.Trigger>
              </Tabs.List>
              
              <div class="tab-content space-y-4 pt-4">
                {#if activeTab === 'login'}
                  <div class="space-y-2">
                    <Label for="username">Username</Label>
                    <Input 
                      id="username" 
                      bind:value={username} 
                      placeholder="Enter your FFXIV username"
                    />
                  </div>
                  <div class="space-y-2">
                    <Label for="password">Password</Label>
                    <Input 
                      id="password" 
                      type="password" 
                      bind:value={password} 
                      placeholder="Enter your password"
                    />
                  </div>
                {:else}
                  <div class="space-y-2">
                    <Label for="otp">One-Time Password</Label>
                    <Input 
                      id="otp" 
                      bind:value={otp} 
                      placeholder="Enter your OTP if enabled"
                    />
                  </div>
                {/if}
              </div>
            </Tabs.Root>
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
  
              <Button variant="outline" on:click={handleNext}>
                Next
              </Button>
            </div>
          </div>
        </div>
      </Card.Content>
    </Card.Root>
  </div>
  

  <style>
    @keyframes fade-in {
      from { opacity: 0; }
    }
  
    @keyframes fade-out {
      to { opacity: 0; }
    }
  
    @keyframes slide-from-right {
      from { transform: translateX(50px); }
    }
  
    @keyframes slide-to-left {
      to { transform: translateX(-50px); }
    }
  
    ::view-transition-old(tab-content) {
      animation: 
        400ms cubic-bezier(0.4, 0, 0.2, 1) both fade-out,
        600ms cubic-bezier(0.4, 0, 0.2, 1) both slide-to-left;
    }
  
    ::view-transition-new(tab-content) {
      animation:
        400ms cubic-bezier(0.4, 0, 0.2, 1) 200ms both fade-in,
        600ms cubic-bezier(0.4, 0, 0.2, 1) both slide-from-right;
    }
  
    .tab-content {
      view-transition-name: tab-content;
    }
  </style>