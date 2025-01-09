<script lang="ts">
  import * as Card from "$lib/components/ui/card/index.js";
  import { Button, buttonVariants } from "$lib/components/ui/button/index.js";
  import { Input } from "$lib/components/ui/input/index.js";
  import { Label } from "$lib/components/ui/label/index.js";
  import * as Form from "$lib/components/ui/form/index.js";
  import * as InputOTP from "$lib/components/ui/input-otp/index.js";
  import { goto } from "$app/navigation";
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from 'svelte';
  import { gameConfig } from '$lib/stores/game-config';
  import { logStore } from '$lib/stores/log-store';
  import { superForm } from "sveltekit-superforms/client";
  import { zodClient } from "sveltekit-superforms/adapters";
  import { z } from "zod";
  import type { SuperForm, SuperValidated } from "sveltekit-superforms";
  import { toast } from "svelte-sonner";
  import { open } from '@tauri-apps/plugin-shell';
  import { Loader2 } from "lucide-svelte";
  import * as Tabs from "$lib/components/ui/tabs/index.js";

  let username = '';
  let password = '';
  let otp = '';
  let activeTab = 'login';
  let headlines: any = null;
  let banners: any[] = [];
  let currentBanner = 0;
  let newsLoaded = false;

  const formSchema = z.object({
    username: z.string().min(1, "Username is required"),
    password: z.string().min(1, "Password is required"),
    otp: z.string().optional()
  });

  type FormSchema = z.infer<typeof formSchema>;

  let formData = {
    username: '',
    password: '',
    otp: ''
  };

  let formErrors: { [key: string]: string[] } = {};

  function validateForm() {
    const result = formSchema.safeParse(formData);
    if (!result.success) {
      formErrors = {};
      result.error.errors.forEach((error) => {
        const path = error.path[0] as string;
        if (!formErrors[path]) {
          formErrors[path] = [];
        }
        formErrors[path].push(error.message);
      });
      return false;
    }
    formErrors = {};
    return true;
  }

  function handleSubmit(event: Event) {
    event.preventDefault();
    if (validateForm()) {
      username = formData.username;
      password = formData.password;
      otp = formData.otp || '';
      handleNext();
    }
  }

  onMount(async () => {
    try {
      headlines = await invoke('get_news', { language: 1, forceNa: false });
      banners = await invoke('get_banners', { language: 1, forceNa: false });
      newsLoaded = true;
      
      // Rotate banners every 8 seconds
      setInterval(() => {
        currentBanner = (currentBanner + 1) % banners.length;
      }, 8000);
    } catch (error) {
      logStore.addLog(`Failed to load news: ${error}`);
    }
  });

  function handleNext() {
    $gameConfig.username = username;
    $gameConfig.password = password;
    $gameConfig.otp = otp;
    logStore.addLog("Credentials saved, navigating to setup page");
    goto("/setup", { replaceState: true });
  }

  function handleBack() {
    logStore.addLog("Navigating back to main page");
    goto("/", { replaceState: true });
  }

  async function handleNewsClick(url: string, event: MouseEvent) {
    event.preventDefault();
    try {
      await open(url);
    } catch (error) {
      logStore.addLog(`Failed to open URL: ${error}`);
    }
  }

  function validateOtpInput(value: string): boolean {
    return /^\d*$/.test(value);
  }

  async function handleRegistration(type: 'general' | 'ffxiv') {
    const baseUrl = type === 'general' 
      ? 'https://secure.square-enix.com/oauth/oa/registligt'
      : 'https://secure.square-enix.com/account/app/svc/ffxivregister';

    const params = new URLSearchParams({
      client_id: 'support',
      ref: 'support_na_login',
      svcgrp: 'Service_SEA',
      response_type: 'code',
      lng: 'en-us',
      redirect_uri: encodeURIComponent(window.location.origin + '/callback')
    });

    try {
      await open(`${baseUrl}?${params.toString()}`);
    } catch (error) {
      logStore.addLog(`Failed to open registration page: ${error}`);
    }
  }
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
          <Card.Description>Enter your FFXIV account details to continue.</Card.Description>
        </div>
      </div>
    </Card.Header>
    
    <Card.Content class="p-0">
      <div class="grid md:grid-cols-2">
        <!-- Login Form -->
        <div class="p-6 md:p-8">
          <div class="flex flex-col gap-6">
            <div class="flex flex-col items-center text-center">
              <h1 class="text-2xl font-bold">Welcome back</h1>
              <p class="text-balance text-muted-foreground">
                Login to your FFXIV account
              </p>
            </div>

            <form on:submit={handleSubmit} class="space-y-4">
              {#if activeTab === 'login'}
                <div class="grid gap-2">
                  <Label for="username">Username</Label>
                  <Input 
                    id="username" 
                    name="username"
                    bind:value={formData.username} 
                    placeholder="Enter your FFXIV username"
                    required
                  />
                  {#if formErrors.username}
                    <p class="text-sm text-destructive">{formErrors.username[0]}</p>
                  {/if}
                </div>
                <div class="grid gap-2">
                  <div class="flex items-center">
                    <Label for="password">Password</Label>
                    <button 
                      type="button"
                      class="ml-auto text-sm underline-offset-2 hover:underline text-muted-foreground"
                      on:click={() => activeTab = 'otp'}
                    >
                      Have an OTP?
                    </button>
                  </div>
                  <Input 
                    id="password" 
                    name="password"
                    type="password" 
                    bind:value={formData.password} 
                    placeholder="Enter your password"
                    required
                  />
                  {#if formErrors.password}
                    <p class="text-sm text-destructive">{formErrors.password[0]}</p>
                  {/if}
                </div>
              {:else}
                <div class="space-y-4">
                  <Label for="otp">One-Time Password</Label>
                  <div class="flex justify-center input-otp-container">
                    <InputOTP.Root 
                      maxlength={6}
                      bind:value={formData.otp}
                      class="otp-root"
                    >
                      {#snippet children({ cells })}
                        <InputOTP.Group class="otp-group">
                          {#each cells.slice(0, 3) as cell}
                            <InputOTP.Slot {cell} class="otp-input" />
                          {/each}
                        </InputOTP.Group>
                        <InputOTP.Separator class="mx-2 text-muted-foreground" />
                        <InputOTP.Group class="otp-group">
                          {#each cells.slice(3, 6) as cell}
                            <InputOTP.Slot {cell} class="otp-input" />
                          {/each}
                        </InputOTP.Group>
                      {/snippet}
                    </InputOTP.Root>
                  </div>
                  <p class="text-sm text-center text-muted-foreground">
                    Please enter your one-time password if enabled.
                  </p>
                  {#if formErrors.otp}
                    <p class="text-sm text-center text-destructive">{formErrors.otp[0]}</p>
                  {/if}
                </div>
                <button 
                  type="button"
                  class="text-sm underline-offset-2 hover:underline text-muted-foreground"
                  on:click={() => activeTab = 'login'}
                >
                  Back to login
                </button>
              {/if}

              <div class="relative my-4">
                <div class="absolute inset-0 flex items-center">
                  <span class="w-full border-t" />
                </div>
                <div class="relative flex justify-center text-xs uppercase">
                  <span class="bg-background px-2 text-muted-foreground">
                    Or create an account
                  </span>
                </div>
              </div>

              <div class="grid grid-cols-2 gap-2">
                <button
                  type="button"
                  class={buttonVariants({ variant: "outline", class: "w-full" })}
                  on:click={() => handleRegistration('general')}
                >
                  Square Enix Account
                </button>
                <button
                  type="button"
                  class={buttonVariants({ variant: "outline", class: "w-full" })}
                  on:click={() => handleRegistration('ffxiv')}
                >
                  FFXIV Trial Account
                </button>
              </div>

              <div class="flex gap-2 pt-4">
                <a href="/" class={buttonVariants({ variant: "outline", class: "flex-1" })} on:click|preventDefault={handleBack}>
                  Back
                </a>
                <button type="submit" class={buttonVariants({ class: "flex-1" })}>
                  Next
                </button>
              </div>
            </form>
          </div>
        </div>

        <!-- News Section -->
        <div class="relative hidden p-6 md:block">
          {#if newsLoaded && banners.length > 0}
            <!-- Banner Card -->
            <Card.Root class="overflow-hidden mb-4 card-banner">
              <div class="relative h-48 banner-container">
                {#key currentBanner}
                  <img 
                    src={banners[currentBanner].lsb_banner} 
                    alt="FFXIV Banner"
                    class="absolute inset-0 h-full w-full object-cover banner-image"
                    on:click={(e) => handleNewsClick(banners[currentBanner].link, e)}
                    style="cursor: pointer;"
                  />
                {/key}
              </div>
            </Card.Root>

            <!-- News Tabs -->
            <Tabs.Root value="topics" class="w-full">
              <Tabs.List class="grid w-full grid-cols-2">
                <Tabs.Trigger value="topics">Topics</Tabs.Trigger>
                <Tabs.Trigger value="updates">Latest Updates</Tabs.Trigger>
              </Tabs.List>
              
              <!-- Topics Tab -->
              <Tabs.Content value="topics">
                {#if headlines?.topics?.length > 0}
                  <div class="space-y-2 pt-4">
                    {#each headlines.topics.slice(0, 3) as item}
                      {#if item.url}
                        <button 
                          on:click={(e) => handleNewsClick(item.url, e)}
                          class="block w-full text-left p-2 rounded-lg hover:bg-muted/50 transition-colors news-item"
                        >
                          <p class="text-sm font-medium">{item.title}</p>
                          <p class="text-xs text-muted-foreground">{new Date(item.date).toLocaleDateString()}</p>
                        </button>
                      {:else}
                        <div class="block w-full text-left p-2 rounded-lg news-item">
                          <p class="text-sm font-medium">{item.title}</p>
                          <p class="text-xs text-muted-foreground">{new Date(item.date).toLocaleDateString()}</p>
                        </div>
                      {/if}
                    {/each}
                  </div>
                {/if}
              </Tabs.Content>

              <!-- Updates Tab -->
              <Tabs.Content value="updates">
                {#if headlines?.news?.length > 0}
                  <div class="space-y-2 pt-4">
                    {#each headlines.news.slice(0, 3) as item}
                      <div class="block w-full text-left p-2 rounded-lg news-item">
                        <p class="text-sm font-medium">{item.title}</p>
                        <p class="text-xs text-muted-foreground">{new Date(item.date).toLocaleDateString()}</p>
                      </div>
                    {/each}
                  </div>
                {/if}
              </Tabs.Content>
            </Tabs.Root>
          {:else}
            <div class="flex items-center justify-center h-full gap-2">
              <Loader2 class="h-4 w-4 animate-spin" />
              <p class="text-muted-foreground">Loading news...</p>
            </div>
          {/if}
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

  /* Login form animations */
  .flex-col {
    animation: slideUp 0.5s ease-out;
  }

  .grid.gap-2 {
    animation: fadeIn 0.5s ease-out;
  }

  /* Banner and news animations */
  .card-banner {
    animation: fadeScale 0.5s ease-out;
  }

  .news-item {
    animation: slideRight 0.3s ease-out;
    animation-fill-mode: both;
  }

  .news-item:nth-child(1) { animation-delay: 0.1s; }
  .news-item:nth-child(2) { animation-delay: 0.2s; }
  .news-item:nth-child(3) { animation-delay: 0.3s; }

  /* OTP input animation */
  .input-otp-container {
    animation: fadeScale 0.3s ease-out;
  }

  /* Button animations */
  button {
    transition: transform 0.2s ease, background-color 0.2s ease;
  }

  button:hover {
    transform: translateY(-1px);
  }

  button:active {
    transform: translateY(0);
  }

  /* Animation keyframes */
  @keyframes slideUp {
    from {
      opacity: 0;
      transform: translateY(20px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }

  @keyframes fadeIn {
    from {
      opacity: 0;
    }
    to {
      opacity: 1;
    }
  }

  @keyframes fadeScale {
    from {
      opacity: 0;
      transform: scale(0.95);
    }
    to {
      opacity: 1;
      transform: scale(1);
    }
  }

  @keyframes slideRight {
    from {
      opacity: 0;
      transform: translateX(-20px);
    }
    to {
      opacity: 1;
      transform: translateX(0);
    }
  }

  /* Banner transition animation */
  .banner-container {
    position: relative;
    overflow: hidden;
  }

  .banner-image {
    animation: bannerTransition 0.8s ease-out;
    will-change: transform, opacity;
  }

  @keyframes bannerTransition {
    0% {
      opacity: 0;
      transform: scale(1.05) translateX(2%);
    }
    100% {
      opacity: 1;
      transform: scale(1) translateX(0);
    }
  }

  /* Enhanced OTP styling */
  :global(.otp-root) {
    display: flex;
    gap: 0.5rem;
    align-items: center;
  }

  :global(.otp-group) {
    display: flex;
    gap: 0.5rem;
  }

  :global(.otp-input) {
    width: 2.5rem;
    height: 2.5rem;
    font-size: 1.25rem;
    border: 1px solid var(--border);
    border-radius: var(--radius);
    text-align: center;
    background: var(--background);
    transition: all 0.2s ease;
  }

  :global(.otp-input:focus) {
    border-color: var(--ring);
    box-shadow: 0 0 0 1px var(--ring);
  }

  :global(.otp-input[data-complete]) {
    animation: numberEntered 0.3s ease-out;
  }

  @keyframes numberEntered {
    0% {
      transform: scale(1.1);
      opacity: 0.5;
    }
    100% {
      transform: scale(1);
      opacity: 1;
    }
  }
</style>