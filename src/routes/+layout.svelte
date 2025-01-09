<script lang="ts">
  import "../app.postcss";
  import { ModeWatcher } from "mode-watcher";
  import PageTransitions from '../components/PageTransitions.svelte';

  export const router = true;
  export const prerender = true;
  export const ssr = false;
  export const trailingSlash = 'never';
</script>

<ModeWatcher defaultMode="dark" />

<div class="relative min-h-screen">
  <PageTransitions>
    <slot />
  </PageTransitions>
</div>

<style>
  :global(body) {
    @apply bg-background text-foreground antialiased;
  }
</style>
