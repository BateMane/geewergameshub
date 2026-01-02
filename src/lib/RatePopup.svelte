<script>
  import { _ } from 'svelte-i18n';
  import { onMount } from 'svelte';
  import { fade, fly } from 'svelte/transition';

  let visible = false;

  onMount(() => {
    // Le popup s'affiche après 2 minutes
    const timer = setTimeout(() => {
      visible = true;
    }, 1000);

    return () => clearTimeout(timer);
  });
</script>

{#if visible}
  <div 
    class="fixed bottom-6 right-6 z-50 w-80 bg-[var(--card-bg)] border border-white/10 shadow-[0_0_30px_rgba(0,0,0,0.5)] rounded-xl p-5 flex flex-col gap-4"
    transition:fly={{ y: 50, duration: 500 }}
  >
    <button 
      class="absolute top-2 right-2 text-gray-500 hover:text-white transition-colors"
      on:click={() => visible = false}
      aria-label="Fermer"
    >
      <svg xmlns="http://www.w3.org/2000/svg" width="20" height="20" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2" stroke-linecap="round" stroke-linejoin="round"><line x1="18" y1="6" x2="6" y2="18"></line><line x1="6" y1="6" x2="18" y2="18"></line></svg>
    </button>

    <p class="text-[var(--text-primary)] text-sm font-medium pr-4 leading-relaxed">
      {$_('rate_popup.text')}
    </p>

    <a 
      href="https://apps.microsoft.com/detail/9N9CF4NLLQZ7?hl=fr&gl=FR&ocid=pdpshare"
      target="_blank"
      rel="noopener noreferrer"
      class="bg-[var(--accent-color)] hover:brightness-110 text-white text-sm font-bold py-2 px-4 rounded-lg text-center transition-all shadow-lg hover:scale-105"
    >
      {$_('rate_popup.btn')}
    </a>
  </div>
{/if}