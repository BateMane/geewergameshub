<script>
  // C'est ici qu'on charge ton fichier CSS pour tout le projet
  import "../input.css";
  
  // Import de la configuration i18n pour qu'elle s'initialise au lancement
  import "../lib/i18n.js"; 
  import { isLoading } from 'svelte-i18n';
  
  // L'import est bon ici
  import RatePopup from '../lib/RatePopup.svelte';
</script>

{#if $isLoading}
  <div class="h-screen w-screen bg-[#121212] text-white flex flex-col items-center justify-center font-sans select-none">
      <div class="w-10 h-10 border-4 border-white/10 border-t-[#5865F2] rounded-full animate-spin mb-4"></div>
      <p class="text-xs text-gray-500 font-bold tracking-widest uppercase">Chargement...</p>
  </div>
{:else}
  <slot />
  <RatePopup />
{/if}