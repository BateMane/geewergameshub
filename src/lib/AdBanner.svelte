<script>
  // @ts-nocheck
  import { invoke } from '@tauri-apps/api/core';

  export let link = ""; 
  export let darkThemeImg = ""; 
  export let lightThemeImg = "";
  export let isDarkTheme = true;

  let visible = true;

  function closeAd() { visible = false; }

  $: currentImage = isDarkTheme ? darkThemeImg : lightThemeImg;
</script>

{#if visible}
  <div class="w-full max-w-[260px] relative group shrink-0 mx-4 flex flex-col justify-center animate-fade-in">
    <div class="absolute -top-2 left-2 bg-[var(--accent-color)] text-white text-[9px] font-bold px-1.5 py-0.5 rounded shadow-sm z-20 uppercase tracking-wider">
      Sponsor
    </div>

    <button on:click|preventDefault|stopPropagation={closeAd} class="absolute -top-1 -right-1 bg-[#333] text-white rounded-full p-0.5 opacity-0 group-hover:opacity-100 transition-opacity z-30 hover:bg-red-500 shadow-md border border-white/10">
        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-3 h-3"><path d="M6.28 5.22a.75.75 0 00-1.06 1.06L8.94 10l-3.72 3.72a.75.75 0 101.06 1.06L10 11.06l3.72 3.72a.75.75 0 101.06-1.06L11.06 10l3.72-3.72a.75.75 0 00-1.06-1.06L10 8.94 6.28 5.22z" /></svg>
    </button>

    <button 
        on:click={() => {
            invoke('open_launcher_page', { id: link, platform: 'Custom' });
        }}
        class="block w-full text-left relative overflow-hidden rounded-xl border border-white/10 shadow-lg hover:shadow-[var(--accent-color)]/20 hover:scale-[1.02] transition-all duration-300 cursor-pointer"
    >
        <div class="absolute inset-0 bg-gradient-to-t from-black/80 via-transparent to-transparent z-10"></div>
        <img src={currentImage} alt="Sponsor" class="w-full h-16 object-cover" /> <div class="absolute bottom-2 left-3 z-10">
            <p class="text-[var(--accent-color)] font-black text-xl leading-none drop-shadow-md">-70%</p>
            <p class="text-white font-black text-[10px] uppercase leading-none drop-shadow-md">On your games?</p>
        </div>
    </button>
  </div>
{/if}

<style>
    .animate-fade-in { animation: fadeIn 0.5s ease-out; }
    @keyframes fadeIn { from { opacity: 0; transform: translateY(5px); } to { opacity: 1; transform: translateY(0); } }
</style>