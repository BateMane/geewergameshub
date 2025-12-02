<script>
  // @ts-nocheck
  import { onMount } from 'svelte';
  import { invoke } from '@tauri-apps/api/core';
  import { convertFileSrc } from '@tauri-apps/api/core';
  import { open } from '@tauri-apps/plugin-dialog';
  import { getCurrentWindow } from '@tauri-apps/api/window';

  // --- ÉTAT GLOBAL ---
  let games = [];
  let selectedPlatform = 'Tout';
  let viewMode = 'grid';
  let sortOrder = 'asc'; 
  let loadingError = "";

  // --- PARAMÈTRES & THÈMES ---
  let showSettingsModal = false;
  let settingsTab = 'general'; // 'general', 'appearance', 'about'
  let availableDrives = [];
  let selectedDrives = []; 

  // Configuration du thème actuel (valeurs par défaut)
  let currentTheme = {
      accent: '#5865F2',
      bg_from: '#121212',
      bg_to: '#0a0a0a',
      card_bg: '#1e1e1e',
      text_primary: '#f3f4f6'
  };

  // Liste des thèmes prédéfinis pour le panneau d'apparence
  const themes = [
    { name: 'Discord Dark', accent: '#5865F2', bg_from: '#202225', bg_to: '#2f3136', card_bg: '#36393f', text_primary: '#dcddde' },
    { name: 'Midnight Blue', accent: '#3b82f6', bg_from: '#0f172a', bg_to: '#1e293b', card_bg: '#1e293b', text_primary: '#e2e8f0' },
    { name: 'Cyberpunk', accent: '#facc15', bg_from: '#18181b', bg_to: '#27272a', card_bg: '#09090b', text_primary: '#f4f4f5' },
    { name: 'Forest', accent: '#10b981', bg_from: '#064e3b', bg_to: '#022c22', card_bg: '#115e59', text_primary: '#d1fae5' },
    { name: 'Blood', accent: '#ef4444', bg_from: '#450a0a', bg_to: '#7f1d1d', card_bg: '#2c0404', text_primary: '#fee2e2' },
    { name: 'Royal', accent: '#a855f7', bg_from: '#2e1065', bg_to: '#4c1d95', card_bg: '#5b21b6', text_primary: '#ede9fe' },
    { name: 'Sunset', accent: '#f97316', bg_from: '#431407', bg_to: '#7c2d12', card_bg: '#9a3412', text_primary: '#ffedd5' },
    { name: 'Steel', accent: '#9ca3af', bg_from: '#111827', bg_to: '#374151', card_bg: '#1f2937', text_primary: '#f9fafb' },
  ];

  // --- MODALES JEUX ---
  let showAddGameModal = false;
  let newGame = { title: "", exePath: "", imgPath: "" };
  let selectedGame = null;
  let gameDetails = null;
  let gameHltb = null; 
  let gameMods = null; 
  let isLoadingDetails = false;

  // --- INITIALISATION ---
  onMount(async () => {
    getCurrentWindow().maximize();
    
    try {
        // 1. Charger les settings
        const settings = await invoke('get_settings');
        
        // 2. Appliquer le thème sauvegardé (ou garder le défaut)
        if(settings.theme && settings.theme.accent) {
            currentTheme = settings.theme;
        }
        
        // 3. Charger les disques
        selectedDrives = settings.selected_drives;
        availableDrives = await invoke('get_system_drives');

        // 4. Appliquer visuellement les variables CSS
        applyTheme();

    } catch(e) {
        console.error("Erreur chargement settings:", e);
    }

    // 5. Scanner les jeux
    await refreshLibrary();
  });

  // Applique les variables CSS au root pour que Tailwind puisse les utiliser
  function applyTheme() {
      const root = document.documentElement;
      root.style.setProperty('--accent-color', currentTheme.accent);
      root.style.setProperty('--bg-from', currentTheme.bg_from);
      root.style.setProperty('--bg-to', currentTheme.bg_to);
      root.style.setProperty('--card-bg', currentTheme.card_bg);
      root.style.setProperty('--text-primary', currentTheme.text_primary);
  }

  // Sélectionne un thème préféfini
  function selectThemePreset(preset) {
      // On déstructure pour créer une copie et éviter de modifier l'objet original
      currentTheme = { 
          accent: preset.accent,
          bg_from: preset.bg_from,
          bg_to: preset.bg_to,
          card_bg: preset.card_bg,
          text_primary: preset.text_primary
      };
      applyTheme();
  }

  // Recharge la bibliothèque de jeux
  async function refreshLibrary() {
    try {
        games = await invoke('get_games');
        loadMissingImages();
    } catch (e) {
        console.error(e);
        loadingError = e.toString();
    }
  }

  // Sauvegarde tout dans le backend Rust
  async function saveSettings() {
      try {
          await invoke('update_settings', {
              theme: currentTheme,
              selectedDrives: selectedDrives
          });
          applyTheme();
          await refreshLibrary(); // Recharger car les disques ont pu changer
          showSettingsModal = false;
      } catch (e) {
          console.error("Erreur sauvegarde:", e);
      }
  }

  // --- LOGIQUE METIER JEUX ---
  
  // Cherche les images manquantes en arrière-plan
  async function loadMissingImages() {
    for (let i = 0; i < games.length; i++) {
        if (games[i].platform !== 'Steam' && (!games[i].image_path || games[i].image_path === "")) {
            try {
                const onlineUrl = await invoke('find_image_online', { title: games[i].title });
                if (onlineUrl && onlineUrl !== "") {
                    games[i].image_path = onlineUrl;
                    const match = onlineUrl.match(/apps\/(\d+)\//);
                    if (match) games[i].steamIdFallback = match[1];
                    games = [...games]; // Trigger reactivity
                }
            } catch (err) {}
        }
    }
  }

  // Filtres et Tris
  $: processedGames = games
      .filter(g => {
          if (selectedPlatform === 'Favoris') return g.is_favorite;
          if (selectedPlatform === 'Tout') return true;
          return g.platform === selectedPlatform;
      })
      .sort((a, b) => {
          if (selectedPlatform !== 'Favoris') {
              if (a.is_favorite && !b.is_favorite) return -1;
              if (!a.is_favorite && b.is_favorite) return 1;
          }
          const titleA = a.title.toLowerCase();
          const titleB = b.title.toLowerCase();
          return sortOrder === 'asc' ? titleA.localeCompare(titleB) : titleB.localeCompare(titleA);
      });

  // Actions
  function play(game) { invoke('launch_game', { id: game.id, platform: game.platform, exePath: game.exe_path || "" }); }
  
  function openInLauncher(game) {
    if (game.platform === 'Custom') {
        const lastSlash = Math.max(game.exe_path.lastIndexOf('\\'), game.exe_path.lastIndexOf('/'));
        const dirPath = lastSlash !== -1 ? game.exe_path.substring(0, lastSlash) : game.exe_path;
        invoke('open_launcher_page', { id: dirPath, platform: 'Custom' });
    } else { invoke('open_launcher_page', { id: game.id, platform: game.platform }); }
  }
  
  function openLink(url) { invoke('open_launcher_page', { id: url, platform: 'Custom' }); }
  
  async function toggleFav(e, game) {
    e.stopPropagation(); game.is_favorite = !game.is_favorite; games = [...games];
    await invoke('toggle_favorite', { gameId: game.id, platform: game.platform });
  }
  
  function pickRandomGame() {
      const candidates = processedGames; if (candidates.length === 0) return;
      const winner = candidates[Math.floor(Math.random() * candidates.length)]; openDetails(winner);
  }

  // Ajout Manuel
  async function pickExe() { const selected = await open({ multiple: false, filters: [{ name: 'Executable', extensions: ['exe', 'lnk', 'url'] }] }); if (selected) newGame.exePath = selected; }
  async function pickImage() { const selected = await open({ multiple: false, filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] }); if (selected) newGame.imgPath = selected; }
  async function saveCustomGame() {
    if(!newGame.title || !newGame.exePath) return;
    await invoke('add_custom_game', { title: newGame.title, exePath: newGame.exePath, imagePath: newGame.imgPath || "" });
    showAddGameModal = false; newGame = { title: "", exePath: "", imgPath: "" }; refreshLibrary();
  }

  // Détails du Jeu
  async function openDetails(game) {
    selectedGame = game; gameDetails = null; gameHltb = null; gameMods = null; isLoadingDetails = true;
    
    // Fetchs parallèles
    invoke('get_hltb', { title: game.title }).then(res => { if(res.main > 0) gameHltb = res; });
    invoke('check_mod_support', { title: game.title }).then(res => { if (res.nexus || res.thunderstore) gameMods = res; });
    
    let targetSteamId = null;
    if (game.platform === 'Steam') targetSteamId = game.id; 
    else if (game.steamIdFallback) targetSteamId = game.steamIdFallback;
    else { try { const url = await invoke('find_image_online', { title: game.title }); const match = url.match(/apps\/(\d+)\//); if (match) targetSteamId = match[1]; } catch(e) {} }
    
    if (targetSteamId) { try { const jsonStr = await invoke('get_steam_details', { steamId: targetSteamId }); const data = JSON.parse(jsonStr); if (data[targetSteamId] && data[targetSteamId].success) { gameDetails = data[targetSteamId].data; } } catch (e) {} }
    isLoadingDetails = false;
  }
  function closeDetails() { selectedGame = null; }

  // Helpers
  function getImgSrc(game) {
    if (game.image_path && game.image_path.startsWith('http')) return game.image_path;
    if (game.image_path && game.image_path.trim() !== "") return convertFileSrc(game.image_path);
    if (game.platform === 'Steam') return `https://cdn.cloudflare.steamstatic.com/steam/apps/${game.id}/library_600x900.jpg`;
    return null;
  }
  function handleImageError(e) { e.target.style.display = 'none'; const p = e.target.parentElement; const f = p.querySelector('.fallback-icon'); if(f) f.style.display = 'flex'; }
  
  // Configurations plateformes (pour les icônes/tags)
  function getPlatformConfig(platform) {
      switch(platform) {
          case 'Steam': return { color: 'bg-[#1a9fff]', text: 'text-[#1a9fff]' };
          case 'Epic': return { color: 'bg-white', text: 'text-white' };
          case 'Ubisoft': return { color: 'bg-[#0070e0]', text: 'text-[#0070e0]' };
          case 'EA': return { color: 'bg-[#ff4747]', text: 'text-[#ff4747]' };
          case 'GOG': return { color: 'bg-[#7d22d3]', text: 'text-[#7d22d3]' };
          case 'Custom': return { color: 'bg-yellow-500', text: 'text-yellow-500' };
          case 'Favoris': return { color: 'bg-yellow-400', text: 'text-yellow-400' };
          default: return { color: 'bg-gray-500', text: 'text-gray-500' };
      }
  }

  // Style des onglets (adapte les couleurs au thème choisi)
  function getTabStyle(platform, selected) {
      const conf = getPlatformConfig(platform);
      if (selected) {
          // Onglet actif : fond couleur texte primaire, texte noir
          if (platform === 'Tout') return "bg-[var(--text-primary)] text-black shadow-lg scale-105 font-bold";
          return `${conf.color} text-black shadow-lg shadow-${conf.color}/20 scale-105 font-bold border-transparent`;
      }
      // Onglet inactif : fond couleur carte
      return `bg-[var(--card-bg)] text-gray-400 border border-white/5 hover:border-white/20 hover:text-white`;
  }
</script>

<main class="h-screen w-screen flex flex-col p-6 text-[var(--text-primary)] select-none font-sans overflow-hidden relative transition-colors duration-500" 
      style="background: linear-gradient(to bottom, var(--bg-from), var(--bg-to));">
  
  <header class="flex flex-col shrink-0 mb-6 z-10 relative">
    <div class="flex items-center justify-between mb-6">
        <div class="flex items-center">
            <div class="w-16 h-16 bg-gradient-to-br from-[#2a2a2a] to-black rounded-2xl flex items-center justify-center mr-5 shadow-2xl ring-1 ring-white/10 p-2">
                <img src="/icon.png" alt="Logo" class="w-full h-full object-contain drop-shadow-[0_0_5px_rgba(255,255,255,0.2)]" />      
            </div>
            <div>
                <h1 class="text-4xl font-black tracking-wider text-[var(--text-primary)] uppercase mb-1">Geewer Game's Hub</h1>
                <p class="text-sm text-gray-400 font-medium">Bienvenue, vous avez <span class="text-[var(--text-primary)] font-bold">{games.length}</span> jeux prêts.</p>
            </div>
        </div>
        
        <div class="flex gap-3">
            <button on:click={() => showSettingsModal = true} class="group bg-[var(--card-bg)] hover:bg-[var(--bg-from)] text-gray-400 hover:text-[var(--accent-color)] border border-white/10 px-3 py-2 rounded-lg transition-all hover:border-[var(--accent-color)] active:scale-95" title="Paramètres">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 transition-transform duration-700 group-hover:rotate-180">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 010 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 010-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281z" />
                    <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
            </button>

            <button on:click={pickRandomGame} 
                class="bg-[var(--card-bg)] hover:bg-[#333] px-3 py-2 rounded-lg font-bold flex items-center shadow-lg transition-all border active:scale-95" 
                style="color: var(--accent-color); border-color: color-mix(in srgb, var(--accent-color), transparent 70%);"
                title="Jeu Aléatoire">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-6 h-6"><path stroke-linecap="round" stroke-linejoin="round" d="M19.5 12c0-1.232-.046-2.453-.138-3.662a4.006 4.006 0 00-3.7-3.7 48.678 48.678 0 00-7.324 0 4.006 4.006 0 00-3.7 3.7c-.017.22-.032.441-.046.662M19.5 12l3-3m-3 3l-3-3m-12 3c0 1.232.046 2.453.138 3.662a4.006 4.006 0 003.7 3.7 48.656 48.656 0 007.324 0 4.006 4.006 0 003.7-3.7c.017-.22.032-.441.046-.662M4.5 12l3 3m-3-3l-3 3" /></svg>
            </button>

            <button on:click={() => showAddGameModal = true} class="bg-[var(--card-bg)] hover:bg-[#333] text-[var(--text-primary)] border border-white/10 px-4 py-2 rounded-lg font-bold flex items-center shadow-lg transition-all hover:border-white/30 active:scale-95">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5 mr-2"><path d="M10.75 4.75a.75.75 0 00-1.5 0v4.5h-4.5a.75.75 0 000 1.5h4.5v4.5a.75.75 0 001.5 0v-4.5h4.5a.75.75 0 000-1.5h-4.5v-4.5z" /></svg> Ajouter
            </button>

            <div class="flex items-center space-x-1 bg-[var(--card-bg)] p-1 rounded-lg border border-white/10 shadow-sm">
                <button aria-label="Grille" on:click={() => viewMode = 'grid'} class={`p-2 rounded-md transition-all ${viewMode === 'grid' ? 'bg-[#333] text-white shadow-sm' : 'text-gray-500 hover:text-white'}`}>
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5"><path fill-rule="evenodd" d="M3 6a3 3 0 013-3h2.25a3 3 0 013 3v2.25a3 3 0 01-3 3H6a3 3 0 01-3-3V6zm9.75 0a3 3 0 013-3H18a3 3 0 013 3v2.25a3 3 0 01-3 3h-2.25a3 3 0 01-3-3V6zM3 15.75a3 3 0 013-3h2.25a3 3 0 013 3V18a3 3 0 01-3 3H6a3 3 0 01-3-3v-2.25zm9.75 0a3 3 0 013-3H18a3 3 0 013 3V18a3 3 0 01-3 3h-2.25a3 3 0 01-3-3v-2.25z" clip-rule="evenodd" /></svg>
                </button>
                <button aria-label="Liste" on:click={() => viewMode = 'list'} class={`p-2 rounded-md transition-all ${viewMode === 'list' ? 'bg-[#333] text-white shadow-sm' : 'text-gray-500 hover:text-white'}`}><svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 12h16.5m-16.5 3.75h16.5M3.75 19.5h16.5M5.625 4.5h12.75a1.875 1.875 0 010 3.75H5.625a1.875 1.875 0 010-3.75z" /></svg></button>
                <div class="w-px h-5 bg-white/10 mx-1"></div>
                <button aria-label="Tri" on:click={() => sortOrder = sortOrder === 'asc' ? 'desc' : 'asc'} class="flex items-center px-3 py-2 text-xs font-bold text-gray-400 hover:text-white hover:bg-[#333] rounded-md transition-all uppercase w-16 justify-between">
                    <span>{sortOrder === 'asc' ? 'A-Z' : 'Z-A'}</span>
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class={`w-4 h-4 transition-transform duration-300 ${sortOrder === 'desc' ? 'rotate-180' : ''}`}><path stroke-linecap="round" stroke-linejoin="round" d="M3 4.5h14.25M3 9h9.75M3 13.5h9.75m4.5-4.5v12m0 0l-3.75-3.75M17.25 21L21 17.25" /></svg>
                </button>
            </div>
        </div>
    </div>

    <div class="flex space-x-3 overflow-x-auto pb-2 custom-scroll">
        {#each ['Tout', 'Favoris', 'Custom', 'Steam', 'Epic', 'Ubisoft', 'EA', 'GOG'] as platform}
            {@const style = getTabStyle(platform, selectedPlatform === platform)}
            {@const conf = getPlatformConfig(platform)}
            <button on:click={() => selectedPlatform = platform} class={`px-5 py-2 rounded-full text-sm transition-all flex items-center ${style}`}>
                {#if platform === 'Favoris'}
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class={`w-4 h-4 mr-2 ${selectedPlatform === platform ? 'text-black' : 'text-yellow-400'}`}><path fill-rule="evenodd" d="M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401z" clip-rule="evenodd" /></svg>
                {:else if platform !== 'Tout'}
                    <span class={`w-2 h-2 rounded-full mr-2 block shadow-sm ${selectedPlatform === platform ? 'bg-black' : conf.color}`}></span>
                {/if}
                {platform === 'Custom' ? 'Perso' : platform.toUpperCase()}
            </button>
        {/each}
    </div>
  </header>
  
  {#if loadingError} <div class="p-4 bg-red-900/20 border border-red-500/50 rounded-lg text-red-200 mb-4 backdrop-blur-sm">Erreur: {loadingError}</div> {/if}

  <div class="flex-1 overflow-y-auto pr-2 pb-6 custom-scroll z-0">
    {#if viewMode === 'grid'}
        <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(180px,1fr))]">
            {#each processedGames as game, i (game.id + game.platform + i)} 
            {@const conf = getPlatformConfig(game.platform)}
            
            <div on:click={() => openDetails(game)} class={`group relative bg-[var(--card-bg)] rounded-xl overflow-hidden hover:-translate-y-1 transition-all duration-300 hover:shadow-[0_10px_20px_-10px_rgba(0,0,0,0.5)] border hover:border-opacity-50 flex flex-col h-80 animate-fade-in cursor-default border-white/5`} style={`--hover-border: ${conf.color}`}>
                
                <button aria-label="Favori" on:click={(e) => toggleFav(e, game)} class={`absolute top-2 right-2 z-50 p-2 rounded-full shadow-lg transition-all backdrop-blur-md ${game.is_favorite ? 'bg-white text-black' : 'bg-black/40 text-white/50 hover:bg-white hover:text-black'}`}>
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401z" clip-rule="evenodd" /></svg>
                </button>

                <div class="relative w-full h-64 bg-[#141414] overflow-hidden">
                    <div class="fallback-icon absolute inset-0 hidden flex-col items-center justify-center text-gray-700 z-0">
                        <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16 opacity-30"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 001.5-1.5V6a1.5 1.5 0 00-1.5-1.5H3.75A1.5 1.5 0 002.25 6v12a1.5 1.5 0 001.5 1.5zm10.5-11.25h.008v.008h-.008V8.25zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" /></svg>
                    </div>
                    {#if getImgSrc(game)} <img src={getImgSrc(game)} alt={game.title} on:error={handleImageError} class="w-full h-full object-cover z-10 relative transition-transform duration-500 group-hover:scale-105" loading="lazy" /> {/if}
                    
                    <div class="absolute inset-0 bg-black/80 opacity-0 group-hover:opacity-100 transition-all duration-300 flex flex-col items-center justify-center backdrop-blur-sm z-40 gap-3 p-4">
                        <button on:click={(e) => { e.stopPropagation(); play(game); }} class="w-full bg-white text-black font-black py-3 px-4 rounded-lg shadow-[0_0_20px_rgba(255,255,255,0.2)] flex items-center justify-center hover:scale-105 transition-transform tracking-wider"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5 mr-2"><path d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z" /></svg> JOUER</button>
                        
                        <div class="flex w-full gap-2">
                            <button on:click={(e) => { e.stopPropagation(); openInLauncher(game); }} class={`flex-1 bg-black/50 text-gray-200 font-semibold py-2 px-2 rounded-lg border border-white/20 transition-all duration-200 flex items-center justify-center text-xs uppercase tracking-wide truncate backdrop-blur-md hover:bg-black/90 ${conf.hoverText}`}>
                                {game.platform === 'Custom' ? 'Dossier' : game.platform}
                            </button>
                            <button on:click={(e) => { e.stopPropagation(); openDetails(game); }} class="bg-black/50 hover:bg-white hover:text-black text-white p-2 rounded-lg border border-white/20 transition-all backdrop-blur-md" title="Détails">
                                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" d="M11.25 11.25l.041-.02a.75.75 0 011.063.852l-.708 2.836a.75.75 0 001.063.853l.041-.021M21 12a9 9 0 11-18 0 9 9 0 0118 0zm-9-3.75h.008v.008H12V8.25z" /></svg>
                            </button>
                        </div>
                    </div>
                </div>
                <div class="p-3 border-t border-white/5 flex-1 flex flex-col justify-center bg-black/20 z-20 relative group-hover:bg-[#222] transition-colors">
                    <h3 class="font-bold text-[var(--text-primary)] truncate text-sm leading-tight mb-1" title={game.title}>{game.title}</h3>
                    <div class="flex items-center"><span class={`w-1.5 h-1.5 rounded-full mr-1.5 ${conf.color}`}></span><p class={`text-[10px] uppercase font-bold tracking-wider ${conf.text}`}>{game.platform}</p></div>
                </div>
            </div>
            {/each}
        </div>
    {:else}
        <div class="flex flex-col space-y-2">
            {#each processedGames as game, i} 
            {@const conf = getPlatformConfig(game.platform)}
            <div on:click={() => openDetails(game)} class={`group flex items-center bg-[var(--card-bg)] hover:brightness-110 rounded-xl p-2 pr-4 transition-all border border-white/5 cursor-pointer animate-fade-in hover:shadow-md hover:${conf.border}`}>
                <button aria-label="Favori" on:click={(e) => toggleFav(e, game)} class={`mr-3 p-2 rounded-full transition-all ${game.is_favorite ? 'text-white bg-white/10' : 'text-gray-600 hover:text-white hover:bg-white/5'}`}>
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401z" clip-rule="evenodd" /></svg>
                </button>
                <div class="w-12 h-16 bg-[#141414] rounded-lg overflow-hidden shrink-0 relative mr-4 border border-white/5">
                    {#if getImgSrc(game)} <img src={getImgSrc(game)} alt="" on:error={handleImageError} class="w-full h-full object-cover" loading="lazy"/> {/if}
                </div>
                <div class="flex-1 min-w-0">
                    <h3 class="font-bold text-[var(--text-primary)] text-sm truncate mb-1">{game.title}</h3>
                    <div class="flex items-center"><span class={`w-1.5 h-1.5 rounded-full mr-2 ${conf.color}`}></span><span class={`text-xs uppercase font-bold ${conf.text}`}>{game.platform}</span></div>
                </div>
                <div class="flex items-center opacity-0 group-hover:opacity-100 transition-all space-x-2">
                    <button aria-label="Jouer" on:click={(e) => { e.stopPropagation(); play(game); }} class="bg-white text-black p-2 rounded-full shadow-[0_0_10px_rgba(255,255,255,0.3)] transition-all hover:scale-110 hover:shadow-[0_0_15px_rgba(255,255,255,0.5)] ml-2"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5"><path d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z" /></svg></button>
                </div>
            </div>
            {/each}
        </div>
    {/if}
  </div>

  {#if showAddGameModal}
    <div class="fixed inset-0 z-[150] flex items-center justify-center p-4 bg-black/90 backdrop-blur-sm animate-fade-in">
        <div class="bg-[var(--card-bg)] w-full max-w-md p-6 rounded-2xl shadow-[0_0_30px_rgba(0,0,0,0.5)] border border-white/10">
            <h2 class="text-2xl font-black text-[var(--text-primary)] mb-6 tracking-wide">AJOUTER UN JEU</h2>
            <div class="space-y-5">
                <label class="block">
                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider ml-1">Titre du jeu</span>
                    <input type="text" bind:value={newGame.title} class="mt-2 w-full bg-black/20 text-[var(--text-primary)] p-3 rounded-xl border border-white/10 focus:border-white/50 outline-none transition-colors font-medium placeholder:text-gray-600" placeholder="Ex: Minecraft" />
                </label>
                <div>
                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider ml-1">Exécutable (.exe)</span>
                    <div class="flex gap-2 mt-2 group">
                        <input type="text" value={newGame.exePath} readonly class="flex-1 bg-black/20 text-gray-500 p-3 rounded-xl border border-white/10 text-sm group-hover:border-white/30 transition-colors truncate" placeholder="Chemin du fichier..." />
                        <button on:click={pickExe} class="bg-[#333] hover:bg-white hover:text-black text-white px-4 rounded-xl border border-white/10 transition-all flex items-center justify-center">
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" /></svg>
                        </button>
                    </div>
                </div>
                <div>
                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider ml-1">Image (Optionnel)</span>
                    <div class="flex gap-2 mt-2 group">
                        <input type="text" value={newGame.imgPath} readonly class="flex-1 bg-black/20 text-gray-500 p-3 rounded-xl border border-white/10 text-sm group-hover:border-white/30 transition-colors truncate" placeholder="Chemin de l'image..." />
                        <button on:click={pickImage} class="bg-[#333] hover:bg-white hover:text-black text-white px-4 rounded-xl border border-white/10 transition-all flex items-center justify-center">
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 001.5-1.5V6a1.5 1.5 0 00-1.5-1.5H3.75A1.5 1.5 0 002.25 6v12a1.5 1.5 0 001.5 1.5zm10.5-11.25h.008v.008h-.008V8.25zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" /></svg>
                        </button>
                    </div>
                </div>
            </div>
            <div class="flex justify-end gap-3 mt-8">
                <button on:click={() => showAddGameModal = false} class="text-gray-400 hover:text-white px-4 py-2 font-bold transition-colors uppercase text-sm tracking-wider">Annuler</button>
                <button on:click={saveCustomGame} class="bg-white text-black hover:bg-gray-200 px-6 py-2 rounded-xl font-black shadow-lg transition-all hover:scale-105 hover:shadow-white/20 uppercase text-sm tracking-wider flex items-center"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4 mr-2"><path d="M10.75 4.75a.75.75 0 00-1.5 0v4.5h-4.5a.75.75 0 000 1.5h4.5v4.5a.75.75 0 001.5 0v-4.5h4.5a.75.75 0 000-1.5h-4.5v-4.5z" /></svg> Ajouter</button>
            </div>
        </div>
    </div>
  {/if}

  {#if showSettingsModal}
    <div class="fixed inset-0 z-[150] flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fade-in">
        <div class="bg-[var(--card-bg)] w-full max-w-4xl h-[70vh] rounded-2xl shadow-[0_0_50px_rgba(0,0,0,0.6)] border border-white/10 flex overflow-hidden">
            
            <div class="w-64 bg-black/20 border-r border-white/5 p-6 flex flex-col gap-2">
                <h2 class="text-xl font-black text-[var(--text-primary)] mb-6 tracking-wide uppercase px-2">Paramètres</h2>
                <button on:click={() => settingsTab = 'general'} class={`text-left px-4 py-3 rounded-lg font-bold transition-all ${settingsTab === 'general' ? 'bg-[var(--accent-color)] text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}>Général</button>
                <button on:click={() => settingsTab = 'appearance'} class={`text-left px-4 py-3 rounded-lg font-bold transition-all ${settingsTab === 'appearance' ? 'bg-[var(--accent-color)] text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}>Apparence</button>
                <button on:click={() => settingsTab = 'about'} class={`text-left px-4 py-3 rounded-lg font-bold transition-all ${settingsTab === 'about' ? 'bg-[var(--accent-color)] text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}>À Propos</button>
                
                <div class="flex-1"></div>
                <button on:click={() => showSettingsModal = false} class="text-left px-4 py-3 rounded-lg font-bold text-gray-500 hover:text-white transition-colors">Fermer</button>
            </div>

            <div class="flex-1 p-8 overflow-y-auto custom-scroll bg-[var(--card-bg)]">
                
                {#if settingsTab === 'general'}
                    <div class="space-y-6 animate-fade-in">
                        <h3 class="text-2xl font-bold text-[var(--text-primary)] mb-4">Disques à scanner</h3>
                        <p class="text-gray-400 text-sm mb-4">Sélectionnez les lecteurs où vos jeux sont installés pour optimiser le scan et ne pas afficher les jeux installés ailleurs.</p>
                        <div class="flex flex-wrap gap-3">
                            {#each availableDrives as drive}
                                <label class="flex items-center cursor-pointer group">
                                    <input type="checkbox" bind:group={selectedDrives} value={drive} class="hidden">
                                    <span class={`px-6 py-3 rounded-xl border-2 transition-all font-bold text-sm flex items-center
                                        ${selectedDrives.includes(drive) 
                                            ? 'border-[var(--accent-color)] bg-[var(--accent-color)]/20 text-white shadow-lg' 
                                            : 'border-white/10 text-gray-400 group-hover:border-white/30'}`
                                    }>
                                        <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4 mr-2"><path d="M7 3a1 1 0 000 2h6a1 1 0 100-2H7z" /><path fill-rule="evenodd" d="M3 6a1 1 0 011-1h12a1 1 0 011 1v10a1 1 0 01-1 1H4a1 1 0 01-1-1V6zM7 11a1 1 0 000 2h6a1 1 0 100-2H7z" clip-rule="evenodd" /></svg>
                                        Disque {drive}
                                    </span>
                                </label>
                            {/each}
                        </div>
                        {#if availableDrives.length === 0}
                            <p class="text-sm text-yellow-400 mt-2 bg-yellow-400/10 p-3 rounded-lg border border-yellow-400/20">Aucun disque détecté automatiquement.</p>
                        {/if}
                    </div>

                {:else if settingsTab === 'appearance'}
                    <div class="space-y-8 animate-fade-in">
                        <div>
                            <h3 class="text-2xl font-bold text-[var(--text-primary)] mb-4">Thèmes Prédéfinis</h3>
                            <div class="grid grid-cols-2 md:grid-cols-3 gap-4">
                                {#each themes as theme}
                                    <button on:click={() => selectThemePreset(theme)} 
                                        class="group relative overflow-hidden rounded-xl border border-white/10 hover:border-white/40 transition-all text-left h-24 shadow-lg hover:shadow-xl hover:scale-[1.02]">
                                        <div class="absolute inset-0" style={`background: linear-gradient(to bottom right, ${theme.bg_from}, ${theme.bg_to})`}></div>
                                        <div class="absolute inset-0 p-4 flex flex-col justify-end">
                                            <div class="flex items-center justify-between">
                                                <span class="font-bold text-white shadow-black drop-shadow-md">{theme.name}</span>
                                                <div class="w-6 h-6 rounded-full border-2 border-white shadow-md" style={`background-color: ${theme.accent}`}></div>
                                            </div>
                                        </div>
                                    </button>
                                {/each}
                            </div>
                        </div>

                        <div class="border-t border-white/10 pt-6">
                            <h3 class="text-2xl font-bold text-[var(--text-primary)] mb-6">Personnalisation Avancée</h3>
                            
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <label class="block space-y-2">
                                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">Couleur d'Accentuation</span>
                                    <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                        <input type="color" bind:value={currentTheme.accent} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/>
                                        <span class="text-gray-300 font-mono text-sm">{currentTheme.accent}</span>
                                    </div>
                                </label>

                                <label class="block space-y-2">
                                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">Couleur des Cartes</span>
                                    <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                        <input type="color" bind:value={currentTheme.card_bg} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/>
                                        <span class="text-gray-300 font-mono text-sm">{currentTheme.card_bg}</span>
                                    </div>
                                </label>

                                <label class="block space-y-2">
                                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">Fond (Haut)</span>
                                    <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                        <input type="color" bind:value={currentTheme.bg_from} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/>
                                        <span class="text-gray-300 font-mono text-sm">{currentTheme.bg_from}</span>
                                    </div>
                                </label>

                                <label class="block space-y-2">
                                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">Fond (Bas)</span>
                                    <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                        <input type="color" bind:value={currentTheme.bg_to} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/>
                                        <span class="text-gray-300 font-mono text-sm">{currentTheme.bg_to}</span>
                                    </div>
                                </label>
                                
                                <label class="block space-y-2">
                                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">Texte Principal</span>
                                    <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                        <input type="color" bind:value={currentTheme.text_primary} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/>
                                        <span class="text-gray-300 font-mono text-sm">{currentTheme.text_primary}</span>
                                    </div>
                                </label>
                            </div>
                        </div>
                    </div>

                {:else if settingsTab === 'about'}
                    <div class="flex flex-col items-center justify-center h-full text-center space-y-6 animate-fade-in">
                         <div class="w-24 h-24 bg-gradient-to-br from-[#2a2a2a] to-black rounded-3xl flex items-center justify-center shadow-2xl ring-1 ring-white/10 p-4">
                            <img src="/icon.png" alt="Logo" class="w-full h-full object-contain" />      
                        </div>
                        <div>
                            <h3 class="text-3xl font-black text-[var(--text-primary)]">Geewer Game's Hub</h3>
                            <p class="text-gray-400 mt-2">Version 1.0.0</p>
                        </div>
                        
                        <div class="flex flex-col gap-3 w-full max-w-sm mt-8">
                            <button 
                                on:click={() => invoke('open_launcher_page', { id: 'mailto:s99ddhhh9@proton.me?subject=Feedback%20GameHub', platform: 'Custom' })}
                                class="bg-[#333] hover:bg-white hover:text-black text-white px-6 py-4 rounded-xl font-bold shadow-lg transition-all flex items-center justify-center"
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5 mr-3"><path fill-rule="evenodd" d="M2.25 2.25A2.25 2.25 0 000 4.5v11A2.25 2.25 0 002.25 18h15.5A2.25 2.25 0 0020 15.5v-11A2.25 2.25 0 0017.75 2.25H2.25zM17.5 5.5v1.75a.75.75 0 01-1.5 0V5.5H4v1.75a.75.75 0 01-1.5 0V5.5H2.25c-.138 0-.25.112-.25.25V15.5c0 .138.112.25.25.25h15.5c.138 0 .25-.112.25-.25V5.75c0-.138-.112-.25-.25-.25H17.5zM6.5 10a1.5 1.5 0 011.5 1.5v2a1.5 1.5 0 01-3 0v-2A1.5 1.5 0 016.5 10zM13.5 10a1.5 1.5 0 011.5 1.5v2a1.5 1.5 0 01-3 0v-2A1.5 1.5 0 0113.5 10z" clip-rule="evenodd" /></svg>
                                Signaler un Bug / Suggestion
                            </button>
                            <button 
                                on:click={() => openLink('https://buymeacoffee.com/geewerrr')}
                                class="bg-[#FFDD00] text-black font-black px-6 py-4 rounded-xl shadow-lg hover:scale-105 transition-all flex items-center justify-center"
                            >
                                <svg class="w-5 h-5 mr-3" fill="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M20.216 6.415l-.132-.666c-.119-.596-.385-1.135-.77-1.566-.751-.841-1.869-1.328-3.006-1.298-2.56.069-12.068 1.583-12.068 1.583-.759.128-1.344.693-1.483 1.445L1.95 10.39c-.141.757.252 1.496.954 1.785a1.997 1.997 0 0 0 2.555-.658l.183-.293c.336 2.543 2.421 4.536 5.007 4.794v2.484H7.917a1 1 0 1 0 0 2h8.166a1 1 0 1 0 0-2h-2.732v-2.496c2.552-.28 4.609-2.254 4.954-4.763l.679.152a2.004 2.004 0 0 0 2.358-1.221 2.003 2.003 0 0 0-1.126-2.463zM16.57 10.202l-.476 2.143c-.219.984-1.157 1.622-2.145 1.458-4.336-.719-7.466-3.548-6.99-5.688.075-.337.254-.639.51-.875 1.324-1.214 7.185-1.494 8.503-1.276.989.164 1.66 1.077 1.441 2.062l-.843 2.176z"/></svg>
                                M'offrir un café
                            </button>
                        </div>
                    </div>
                {/if}

            </div>
            
            <div class="absolute bottom-6 right-8">
                <button on:click={saveSettings} class="bg-[var(--accent-color)] text-white hover:brightness-110 px-6 py-3 rounded-xl font-black shadow-lg shadow-[var(--accent-color)]/20 transition-all hover:scale-105 uppercase text-sm tracking-wider">
                    Sauvegarder
                </button>
            </div>
        </div>
    </div>
  {/if}

  {#if selectedGame}
    <div class="fixed inset-0 z-[100] flex items-center justify-center p-8 animate-fade-in">
        <div class="absolute inset-0 bg-black/90 backdrop-blur-md" on:click={closeDetails}></div>
        <div class="bg-[#1e1e1e] w-full max-w-4xl h-full max-h-[600px] rounded-2xl shadow-[0_0_50px_rgba(0,0,0,0.8)] relative flex overflow-hidden ring-1 ring-white/10">
            <button aria-label="Fermer" on:click={closeDetails} class="absolute top-4 right-4 z-50 bg-black/50 text-white p-2 rounded-full hover:bg-white hover:text-black transition-colors backdrop-blur-md border border-white/10">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
            </button>
            {#if isLoadingDetails}
                <div class="w-full h-full flex items-center justify-center text-white flex-col">
                    <div class="w-12 h-12 border-4 border-white/20 border-t-white rounded-full animate-spin mb-4"></div>
                    <p class="text-gray-400 font-medium tracking-wider uppercase text-sm">Chargement...</p>
                </div>
            {:else if gameDetails}
                <div class="absolute inset-0 z-0">
                    <img src={gameDetails.background} alt="" class="w-full h-full object-cover opacity-10 saturate-0" />
                    <div class="absolute inset-0 bg-gradient-to-t from-[#1e1e1e] via-[#1e1e1e]/90 to-transparent"></div>
                </div>
                <div class="relative z-10 flex w-full h-full">
                    <div class="w-1/3 p-8 flex flex-col justify-start border-r border-white/5">
                        <img src={gameDetails.header_image} alt="" class="w-full rounded-xl shadow-2xl ring-1 ring-white/10 mb-6 object-cover" />
                        <div class="space-y-4">
                            <div class="flex justify-between text-sm border-b border-white/10 pb-3">
                                <span class="text-gray-500 font-medium uppercase tracking-wider text-xs">Développeur</span>
                                <span class="text-white font-bold">{gameDetails.developers ? gameDetails.developers[0] : 'Inconnu'}</span>
                            </div>
                            <div class="flex justify-between text-sm border-b border-white/10 pb-3">
                                <span class="text-gray-500 font-medium uppercase tracking-wider text-xs">Date</span>
                                <span class="text-white font-bold">{gameDetails.release_date?.date || 'Inconnue'}</span>
                            </div>
                            {#if gameDetails.metacritic}
                            <div class="flex justify-between items-center text-sm border-b border-white/10 pb-3">
                                <span class="text-gray-500 font-medium uppercase tracking-wider text-xs">Metacritic</span>
                                <span class={`font-black px-3 py-1 rounded text-xs ${gameDetails.metacritic.score >= 80 ? 'bg-white text-black' : 'bg-[#333] text-white border border-white/20'}`}>{gameDetails.metacritic.score}</span>
                            </div>
                            {/if}
                        </div>
                        {#if gameMods}
                            <div class="mt-6 pt-4 border-t border-white/10 animate-fade-in">
                                <span class="text-gray-500 font-black uppercase tracking-widest text-[10px] block mb-3">MODS DISPONIBLES</span>
                                <div class="flex gap-2">
                                    {#if gameMods.thunderstore}
                                        <button on:click={() => openLink(gameMods.thunderstore)} class="bg-[#ff0000]/10 border border-[#ff0000]/30 hover:bg-[#ff0000] text-red-400 hover:text-white px-3 py-2 rounded text-xs font-bold transition-all flex-1">THUNDERSTORE</button>
                                    {/if}
                                    {#if gameMods.nexus}
                                        <button on:click={() => openLink(gameMods.nexus)} class="bg-[#da8e35]/10 border border-[#da8e35]/30 hover:bg-[#da8e35] text-orange-400 hover:text-white px-3 py-2 rounded text-xs font-bold transition-all flex-1">NEXUS MODS</button>
                                    {/if}
                                </div>
                            </div>
                        {/if}
                        {#if gameHltb}
                            <div class="mt-6 pt-4 border-t border-white/10 animate-fade-in">
                                <div class="flex items-center mb-3">
                                    <span class="text-gray-500 font-black uppercase tracking-widest text-[10px] mr-2">Temps de jeu</span>
                                    <span class="text-[10px] bg-[#2f3136] text-gray-400 px-1 rounded border border-white/5">HLTB</span>
                                </div>
                                <div class="space-y-2">
                                    <div class="flex justify-between items-center bg-[#141414] p-2 rounded border border-white/5">
                                        <span class="text-gray-400 text-xs">Histoire</span>
                                        <span class="text-white font-bold text-sm">{gameHltb.main}h</span>
                                    </div>
                                    <div class="flex justify-between items-center bg-[#141414] p-2 rounded border border-white/5">
                                        <span class="text-gray-400 text-xs">Extra</span>
                                        <span class="text-[#5865F2] font-bold text-sm">{gameHltb.main_extra}h</span>
                                    </div>
                                    <div class="flex justify-between items-center bg-[#141414] p-2 rounded border border-white/5">
                                        <span class="text-gray-400 text-xs">100%</span>
                                        <span class="text-yellow-500 font-bold text-sm">{gameHltb.completionist}h</span>
                                    </div>
                                </div>
                            </div>
                        {/if}
                    </div>
                    <div class="w-2/3 p-8 overflow-y-auto custom-scroll">
                        <h1 class="text-4xl font-black text-white mb-2 tracking-wide uppercase">{selectedGame.title}</h1>
                        <div class="flex items-center mb-8">
                            <span class={`w-1.5 h-1.5 rounded-full mr-2 ${getPlatformConfig(selectedGame.platform).color}`}></span>
                            <p class={`font-bold uppercase tracking-widest text-xs ${getPlatformConfig(selectedGame.platform).text}`}>Source: {selectedGame.platform}</p>
                        </div>
                        <div class="flex gap-4 mb-8">
                            <button on:click={() => play(selectedGame)} class="flex-1 bg-white text-black hover:bg-gray-200 py-4 rounded-xl font-black shadow-[0_0_20px_rgba(255,255,255,0.15)] flex items-center justify-center text-lg hover:scale-[1.02] transition-all tracking-wider uppercase">
                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-6 h-6 mr-2"><path d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z" /></svg> LANCER
                            </button>
                            <button on:click={() => openInLauncher(selectedGame)} class={`hover:bg-white hover:text-black px-8 py-4 rounded-xl border border-white/10 font-bold flex items-center transition-all uppercase tracking-wider text-sm ${getPlatformConfig(selectedGame.platform).color} bg-opacity-20 text-white`}>
                                {selectedGame.platform === 'Custom' ? 'DOSSIER' : 'OUVRIR'}
                            </button>
                        </div>
                        <div class="prose prose-invert prose-sm max-w-none text-gray-300 leading-relaxed">{@html gameDetails.short_description}</div>
                    </div>
                </div>
            {:else}
                <div class="w-full h-full flex flex-col items-center justify-center relative z-10 p-10 text-center">
                    <h1 class="text-5xl font-black text-white mb-6 uppercase tracking-wide">{selectedGame.title}</h1>
                    <p class="text-gray-500 mb-12 max-w-md font-medium">Aucune information supplémentaire disponible pour ce jeu.</p>
                    <div class="flex gap-5">
                        <button on:click={() => play(selectedGame)} class="bg-white text-black hover:bg-gray-200 py-4 px-10 rounded-xl font-black shadow-lg flex items-center justify-center text-lg hover:scale-105 transition-transform uppercase tracking-wider"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5 mr-2"><path d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z" /></svg> LANCER</button>
                        <button on:click={() => openInLauncher(selectedGame)} class="bg-[#333] hover:bg-white hover:text-black text-white px-8 py-4 rounded-xl border border-white/10 font-bold transition-all uppercase tracking-wider text-sm flex items-center">
                            {selectedGame.platform === 'Custom' ? 'DOSSIER' : 'OUVRIR'}
                        </button>
                    </div>
                </div>
            {/if}
        </div>
    </div>
  {/if}

</main>

<style>
  .custom-scroll::-webkit-scrollbar { width: 5px; height: 5px; }
  .custom-scroll::-webkit-scrollbar-track { background: rgba(255, 255, 255, 0.05); border-radius: 10px; }
  .custom-scroll::-webkit-scrollbar-thumb { background: rgba(255, 255, 255, 0.2); border-radius: 10px; }
  .custom-scroll::-webkit-scrollbar-thumb:hover { background: rgba(255, 255, 255, 0.4); }
  .animate-fade-in { animation: fadeIn 0.4s cubic-bezier(0.16, 1, 0.3, 1) forwards; }
  @keyframes fadeIn { from { opacity: 0; transform: translateY(10px) scale(0.98); } to { opacity: 1; transform: translateY(0) scale(1); } }
</style>