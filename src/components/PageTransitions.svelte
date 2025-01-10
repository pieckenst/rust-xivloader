<!-- src/component/PageTransitions.svelte -->
<script lang="ts">
  import { onNavigate } from '$app/navigation';
  import { fade } from 'svelte/transition';
  
  let key = '';
  
  onNavigate((navigation) => {
    key = navigation.to?.url.pathname ?? '';
    
    if (!document.startViewTransition) return;

    return new Promise((resolve) => {
      document.startViewTransition(async () => {
        resolve();
        await navigation.complete;
      });
    });
  });
</script>

{#key key}
  <div 
    in:fade={{ duration: 400, delay: 200 }}
    out:fade={{ duration: 400 }}
    class="fixed inset-0 w-full"
  >
    <slot />
  </div>
{/key}

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

  :root::view-transition-old(page-content) {
    animation: 
      400ms cubic-bezier(0.4, 0, 0.2, 1) both fade-out,
      600ms cubic-bezier(0.4, 0, 0.2, 1) both slide-to-left;
  }

  :root::view-transition-new(page-content) {
    animation:
      400ms cubic-bezier(0.4, 0, 0.2, 1) 200ms both fade-in,
      600ms cubic-bezier(0.4, 0, 0.2, 1) both slide-from-right;
  }

  div {
    view-transition-name: page-content;
    overflow: hidden;
  }
</style>