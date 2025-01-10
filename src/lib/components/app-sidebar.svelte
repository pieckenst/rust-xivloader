<script lang="ts">
  import * as Sidebar from "$lib/components/ui/sidebar/index.js";
  import { Home, Settings, Download, FileText } from "lucide-svelte";

  const items = [
    {
      title: "Home",
      url: "/",
      icon: Home,
    },
    {
      title: "Login",
      url: "/login",
      icon: FileText,
    },
    {
      title: "Setup",
      url: "/setup",
      icon: Settings,
    },
    {
      title: "Downloads",
      url: "#",
      icon: Download,
    }
  ];
</script>

<Sidebar.Root>
  <Sidebar.Content>
    <div class="flex h-16 items-center gap-2 border-b px-4">
      <div class="flex h-8 w-8 items-center justify-center rounded-lg bg-primary">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round" class="h-4 w-4 text-primary-foreground">
          <path d="M12 2L2 7l10 5 10-5-10-5zM2 17l10 5 10-5M2 12l10 5 10-5"/>
        </svg>
      </div>
      <div class="font-semibold">XIV Loader</div>
    </div>
    <div class="flex-1 overflow-auto">
      <nav class="grid gap-1 p-2">
        {#each items as item (item.title)}
          <a 
            href={item.url}
            class="flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors hover:bg-muted"
          >
            <svelte:component this={item.icon} class="h-4 w-4" />
            {item.title}
          </a>
        {/each}
      </nav>
    </div>
    <div class="mt-auto border-t px-4 py-3">
      <p class="text-xs text-muted-foreground">Version 1.0.0</p>
    </div>
  </Sidebar.Content>
</Sidebar.Root> 