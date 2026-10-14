<script>
  // @ts-nocheck
  import { onMount } from 'svelte';
  import { invoke, convertFileSrc } from '@tauri-apps/api/core';
  import { open, ask } from '@tauri-apps/plugin-dialog';
  import { getCurrentWindow } from '@tauri-apps/api/window';
  import { getVersion } from '@tauri-apps/api/app';

  // Autostart : commandes Rust get_autostart / set_autostart (compatibles MSIX)
  
  // Imports Traduction
  import { _, locale } from 'svelte-i18n';

  // Imports Publicité (Sponsor)
  import AdBanner from '$lib/AdBanner.svelte';

  // --- ÉTAT GLOBAL ---
  let games = [];
  let selectedPlatform = 'all'; 
  let viewMode = 'grid';
  let sortOrder = 'asc'; // 'asc' | 'desc' | 'recent'
  let loadingError = "";
  let isScanning = true;
  let searchQuery = "";
  let searchInput;
  let appVersion = "";
  let toastMsg = "";
  let toastTimer;
  // URLs d'images en échec : on passe au candidat suivant (HD -> local -> SD -> header)
  let brokenImgs = {};
  // Titre -> appid Steam trouvé en ligne pour les jeux hors Steam (map réactive explicite :
  // muter les objets jeu ne rafraîchit pas les {@const} en Svelte 5 mode legacy)
  let onlineArt = {};
  // appid -> { cover, wide, hero } : vrais chemins HD renvoyés par Steam
  let steamArt = {};
  let themeBeforeSettings = null;
  let lightboxUrl = null;

  // --- PARAMÈTRES & THÈMES ---
  let showSettingsModal = false;
  let settingsTab = 'general';
  let availableDrives = [];
  let selectedDrives = []; 

  // --- AUTOSTART STATE ---
  let autostartEnabled = false;
  let showAutostartPopup = false;

  // Configuration du thème actuel
  let currentTheme = {
      accent: '#5865F2',
      bg_from: '#121212',
      bg_to: '#0a0a0a',
      card_bg: '#1e1e1e',
      text_primary: '#f3f4f6'
  };

  // --- DETECTION THEME CLAIR/SOMBRE ---
  function isColorDark(hexColor) {
      if (!hexColor) return true;
      const c = hexColor.substring(1);
      const rgb = parseInt(c, 16);
      const r = (rgb >> 16) & 0xff;
      const g = (rgb >>  8) & 0xff;
      const b = (rgb >>  0) & 0xff;
      const luma = 0.2126 * r + 0.7152 * g + 0.0722 * b; 
      return luma < 128; 
  }

  $: isAppDark = isColorDark(currentTheme.bg_from);

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
        const settings = await invoke('get_settings');
        if(settings.theme && settings.theme.accent) currentTheme = settings.theme;
        selectedDrives = settings.selected_drives;
        availableDrives = await invoke('get_system_drives');
        applyTheme();

        autostartEnabled = await invoke('get_autostart');
        const hidePopup = localStorage.getItem('hideAutostartPopup');
        if (!autostartEnabled && hidePopup !== 'true') showAutostartPopup = true;

        const savedLang = localStorage.getItem('user_lang');
        if (savedLang) $locale = savedLang;

        const savedSort = localStorage.getItem('sort_order');
        if (['asc', 'desc', 'recent'].includes(savedSort)) sortOrder = savedSort;

        appVersion = await getVersion();
    } catch(e) {
        console.error("Erreur init:", e);
    }
    await refreshLibrary();
  });

  // --- LANGUE ---
  function toggleLanguage() {
      $locale = $locale === 'fr' ? 'en' : 'fr';
      localStorage.setItem('user_lang', $locale);
  }

  // --- AUTOSTART ---
  async function toggleAutostart() {
      try {
          autostartEnabled = await invoke('set_autostart', { enabled: !autostartEnabled });
      } catch (e) { console.error("Erreur autostart:", e); }
  }

  function closeAutostartPopup(dontAskAgain) {
      showAutostartPopup = false;
      if (dontAskAgain) localStorage.setItem('hideAutostartPopup', 'true');
  }

  // --- THEME ---
  function applyTheme() {
      const root = document.documentElement;
      root.style.setProperty('--accent-color', currentTheme.accent);
      root.style.setProperty('--bg-from', currentTheme.bg_from);
      root.style.setProperty('--bg-to', currentTheme.bg_to);
      root.style.setProperty('--card-bg', currentTheme.card_bg);
      root.style.setProperty('--text-primary', currentTheme.text_primary);
  }

  function selectThemePreset(preset) {
      currentTheme = { accent: preset.accent, bg_from: preset.bg_from, bg_to: preset.bg_to, card_bg: preset.card_bg, text_primary: preset.text_primary };
      applyTheme();
  }

  // --- CORE ---
  async function refreshLibrary() {
    isScanning = true;
    loadingError = "";
    try {
        games = await invoke('get_games');
        loadSteamArt();
        loadMissingImages().then(loadSteamArt);
    } catch (e) {
        console.error(e);
        loadingError = e.toString();
    }
    isScanning = false;
  }

  function openSettings() {
      themeBeforeSettings = { ...currentTheme };
      showSettingsModal = true;
  }

  // Fermer sans sauvegarder annule l'aperçu du thème
  function closeSettings() {
      if (themeBeforeSettings) { currentTheme = themeBeforeSettings; applyTheme(); }
      showSettingsModal = false;
  }

  async function saveSettings() {
      try {
          await invoke('update_settings', { theme: currentTheme, selectedDrives: selectedDrives });
          applyTheme();
          themeBeforeSettings = null;
          showSettingsModal = false;
          await refreshLibrary();
      } catch (e) { console.error("Erreur sauvegarde:", e); }
  }

  function showToast(msg) {
      toastMsg = msg;
      clearTimeout(toastTimer);
      toastTimer = setTimeout(() => toastMsg = "", 3000);
  }

  function cycleSort() {
      sortOrder = sortOrder === 'asc' ? 'desc' : sortOrder === 'desc' ? 'recent' : 'asc';
      localStorage.setItem('sort_order', sortOrder);
  }

  function handleKeydown(e) {
      if (e.key === 'Escape') {
          if (lightboxUrl) lightboxUrl = null;
          else if (selectedGame) closeDetails();
          else if (showSettingsModal) closeSettings();
          else if (showAddGameModal) showAddGameModal = false;
          else if (searchQuery) searchQuery = "";
      } else if (e.ctrlKey && e.key.toLowerCase() === 'f') {
          e.preventDefault();
          searchInput?.focus();
      } else if (e.key === 'F5') {
          e.preventDefault();
          refreshLibrary();
      }
  }

  // --- IMAGES & LOGIQUE ---
  // Jeux hors Steam : on cherche leur page Steam pour récupérer les visuels HD
  // (le backend met le résultat en cache, seuls les nouveaux jeux interrogent Steam)
  async function loadMissingImages() {
    for (const game of games) {
        if (game.platform === 'Steam' || onlineArt[game.title]) continue;
        if (game.platform === 'Custom' && game.image_path) continue;
        try {
            const url = await invoke('find_image_online', { title: game.title });
            const match = url && url.match(/apps\/(\d+)\//);
            if (match) onlineArt = { ...onlineArt, [game.title]: match[1] };
        } catch (err) {}
    }
  }

  async function loadSteamArt() {
    const ids = [...new Set(games.map(g => steamIdOf(g, onlineArt)).filter(id => id && !steamArt[id]))];
    if (!ids.length) return;
    try {
        steamArt = { ...steamArt, ...(await invoke('get_steam_art', { appIds: ids })) };
    } catch (e) { console.error("Visuels Steam:", e); }
  }

  const normalize = (s) => s.toLowerCase().normalize('NFD').replace(/\p{Diacritic}/gu, '');

  $: visibleGames = games.filter(g => !g.is_hidden);
  $: hiddenCount = games.length - visibleGames.length;
  $: platformCounts = visibleGames.reduce((acc, g) => { acc[g.platform] = (acc[g.platform] || 0) + 1; return acc; }, {});
  $: favCount = visibleGames.filter(g => g.is_favorite).length;
  $: recentGames = visibleGames.filter(g => g.last_played > 0).sort((a, b) => b.last_played - a.last_played).slice(0, 6);
  $: tabs = ['all', 'fav', ...['Custom', 'Steam', 'Epic', 'Ubisoft', 'EA', 'GOG'].filter(p => platformCounts[p]), ...(hiddenCount ? ['hidden'] : [])];
  // Si l'onglet courant disparaît (plus de jeux cachés, etc.), on revient sur "Tout"
  $: if (!isScanning && !tabs.includes(selectedPlatform)) selectedPlatform = 'all';
  $: showRecentRow = selectedPlatform === 'all' && !searchQuery && recentGames.length > 0;

  $: processedGames = (selectedPlatform === 'hidden' ? games.filter(g => g.is_hidden) : visibleGames)
      .filter(g => {
          if (selectedPlatform === 'fav') return g.is_favorite;
          if (selectedPlatform === 'all' || selectedPlatform === 'hidden') return true;
          return g.platform === selectedPlatform;
      })
      .filter(g => !searchQuery || normalize(g.title).includes(normalize(searchQuery)))
      .sort((a, b) => {
          if (sortOrder === 'recent' && a.last_played !== b.last_played) return b.last_played - a.last_played;
          if (selectedPlatform !== 'fav' && sortOrder !== 'recent') {
              if (a.is_favorite && !b.is_favorite) return -1;
              if (!a.is_favorite && b.is_favorite) return 1;
          }
          const titleA = a.title.toLowerCase();
          const titleB = b.title.toLowerCase();
          return sortOrder === 'desc' ? titleB.localeCompare(titleA) : titleA.localeCompare(titleB);
      });

  function timeAgo(ts, lang) {
      if (!ts) return '';
      const rtf = new Intl.RelativeTimeFormat(lang || 'fr', { numeric: 'auto' });
      const diff = ts - Date.now() / 1000;
      const steps = [[60, 'second'], [3600, 'minute'], [86400, 'hour'], [604800, 'day'], [2629800, 'week'], [31557600, 'month'], [Infinity, 'year']];
      let unit = 1;
      for (const [limit, name] of steps) {
          if (Math.abs(diff) < limit) return rtf.format(Math.round(diff / unit), name);
          unit = limit;
      }
  }

  async function play(game) {
    showToast(`${$_('toast.launching')} ${game.title}…`);
    try {
        game.last_played = await invoke('launch_game', { id: game.id, platform: game.platform, exePath: game.exe_path || "" });
        games = [...games];
    } catch (e) { console.error(e); }
  }

  async function toggleHidden(game) {
    game.is_hidden = await invoke('toggle_hidden', { gameId: game.id, platform: game.platform });
    games = [...games];
    showToast(game.is_hidden ? $_('toast.hidden') : $_('toast.unhidden'));
    if (game.is_hidden) closeDetails();
  }

  async function removeCustomGame(game) {
    const ok = await ask($_('details.remove_confirm'), { title: game.title, kind: 'warning' });
    if (!ok) return;
    await invoke('remove_custom_game', { gameId: game.id });
    closeDetails();
    await refreshLibrary();
  }

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

  async function pickExe() { const selected = await open({ multiple: false, filters: [{ name: 'Executable', extensions: ['exe', 'lnk', 'url'] }] }); if (selected) newGame.exePath = selected; }
  async function pickImage() { const selected = await open({ multiple: false, filters: [{ name: 'Image', extensions: ['png', 'jpg', 'jpeg', 'webp'] }] }); if (selected) newGame.imgPath = selected; }
  async function saveCustomGame() {
    if(!newGame.title.trim() || !newGame.exePath) return;
    try {
        await invoke('add_custom_game', { title: newGame.title, exePath: newGame.exePath, imagePath: newGame.imgPath || "" });
    } catch (e) { showToast(`${$_('error')}: ${e}`); return; }
    showToast(`${newGame.title} ${$_('toast.added')}`);
    showAddGameModal = false; newGame = { title: "", exePath: "", imgPath: "" }; refreshLibrary();
  }

  async function openDetails(game) {
    selectedGame = game; 
    gameDetails = null; 
    gameHltb = null; 
    gameMods = null; 
    isLoadingDetails = true;
    
    // Récupération des infos annexes
    invoke('get_hltb', { title: game.title }).then(res => { if(res.main > 0) gameHltb = res; });
    invoke('check_mod_support', { title: game.title }).then(res => { if (res.nexus || res.thunderstore) gameMods = res; });
    
    let targetSteamId = null;
    if (game.platform === 'Steam') targetSteamId = game.id; 
    else if (onlineArt[game.title]) targetSteamId = onlineArt[game.title];
    else { 
        try { 
            const url = await invoke('find_image_online', { title: game.title }); 
            const match = url.match(/apps\/(\d+)\//); 
            if (match) targetSteamId = match[1]; 
        } catch(e) {} 
    }
    
    if (targetSteamId) { 
        try { 
            const steamLang = $locale && $locale.toLowerCase().startsWith('fr') ? 'french' : 'english';
            const jsonStr = await invoke('get_steam_details', { 
                steamId: targetSteamId, 
                language: steamLang 
            }); 
            const data = JSON.parse(jsonStr); 
            if (data[targetSteamId] && data[targetSteamId].success) { 
                gameDetails = data[targetSteamId].data; 
            } 
        } catch (e) { console.error("Erreur chargement détails Steam:", e); } 
    }
    isLoadingDetails = false;
  }
  function closeDetails() { selectedGame = null; lightboxUrl = null; }

  // La description Steam est du HTML externe : on n'en garde que le texte (pas de {@html})
  function stripHtml(html) {
    return new DOMParser().parseFromString(html || '', 'text/html').body.textContent || '';
  }

  // --- IMAGES HD : liste de candidats, du meilleur au plus sûr ---
  const CDN = 'https://cdn.cloudflare.steamstatic.com/steam/apps';
  const steamIdOf = (game, art) => game.platform === 'Steam' ? game.id : art[game.title];
  const localSrc = (path) => path && !path.startsWith('http') ? convertFileSrc(path) : null;
  const firstWorking = (list, broken) => list.find(u => u && !broken[u]) || null;

  // Ordre : visuel HD officiel -> fichier local (cache Steam / dossier du jeu) -> anciennes URL statiques
  // (ces dernières donnent une image grise pour les jeux récents, d'où leur dernière place)
  function getImgSrc(game, broken, art, sart) {
    const sid = steamIdOf(game, art);
    const local = localSrc(game.image_path);
    const list = [];
    if (game.platform === 'Custom') list.push(local); // l'image choisie par l'utilisateur passe en premier
    if (sid) list.push(sart[sid]?.cover);
    list.push(local);
    if (sid) list.push(`${CDN}/${sid}/library_600x900_2x.jpg`, `${CDN}/${sid}/library_600x900.jpg`, `${CDN}/${sid}/header.jpg`);
    return firstWorking(list, broken);
  }

  function getWideSrc(game, broken, art, sart) {
    const sid = steamIdOf(game, art);
    const local = localSrc(game.image_path);
    return firstWorking(sid ? [sart[sid]?.wide, `${CDN}/${sid}/capsule_616x353.jpg`, local] : [local], broken);
  }

  function getHeroSrc(game, details, broken, art, sart) {
    const sid = steamIdOf(game, art);
    return firstWorking([sid && sart[sid]?.hero, localSrc(game.hero_path), sid && `${CDN}/${sid}/library_hero.jpg`, details?.background], broken);
  }

  function markBroken(url) { brokenImgs[url] = true; brokenImgs = brokenImgs; }

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

  function getTabStyle(platform, selected) {
      const conf = getPlatformConfig(platform);
      if (selected) {
          if (platform === 'all' || platform === 'hidden') return "bg-[var(--text-primary)] text-black shadow-lg scale-105 font-bold";
          return `${conf.color} text-black shadow-lg scale-105 font-bold border-transparent`;
      }
      return `bg-[var(--card-bg)] text-gray-400 border border-white/5 hover:border-white/20 hover:text-white`;
  }
</script>

<svelte:window on:keydown={handleKeydown} />

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
                <p class="text-sm text-gray-400 font-medium">
                    {$_('header.welcome')} <span class="text-[var(--text-primary)] font-bold">{visibleGames.length}</span> {$_('header.games_ready')}
                </p>
            </div>
        </div>
        
        <div class="flex-1 flex justify-center px-4">
             <AdBanner 
                link="https://www.instant-gaming.com/?igr=gamer-dc3821"
                darkThemeImg="/banner_light.png" 
                lightThemeImg="/banner_dark.png"
                isDarkTheme={isAppDark}
             />
        </div>

        <div class="flex gap-3 items-center">
            <button on:click={toggleLanguage} class="flex items-center justify-center gap-2 border border-white/10 px-3 py-2 rounded-lg bg-[var(--card-bg)] hover:bg-[#333] transition-all hover:scale-105 shadow-md active:scale-95" title={$locale === 'fr' ? 'Switch to English' : 'Passer en Français'}>
                {#if $locale === 'fr'}
                    <svg class="w-6 h-4 rounded shadow-sm" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 3 2"><path fill="#EC1920" d="M0 0h3v2H0z"/><path fill="#fff" d="M0 0h2v2H0z"/><path fill="#051440" d="M0 0h1v2H0z"/></svg>
                    <span class="text-xs font-bold text-gray-300">FR</span>
                {:else}
                    <svg class="w-6 h-4 rounded shadow-sm" xmlns="http://www.w3.org/2000/svg" viewBox="0 0 60 30"><clipPath id="a"><path d="M0 0v30h60V0z"/></clipPath><path d="M0 0v30h60V0z" fill="#012169"/><path d="M0 0l60 30m0-30L0 30" stroke="#fff" stroke-width="6"/><path d="M0 0l60 30m0-30L0 30" clip-path="url(#a)" stroke="#C8102E" stroke-width="4"/><path d="M30 0v30M0 15h60" stroke="#fff" stroke-width="10"/><path d="M30 0v30M0 15h60" stroke="#C8102E" stroke-width="6"/></svg>
                    <span class="text-xs font-bold text-gray-300">EN</span>
                {/if}
            </button>

            <button on:click={openSettings} class="group bg-[var(--card-bg)] hover:bg-[var(--bg-from)] text-gray-400 hover:text-[var(--accent-color)] border border-white/10 px-3 py-2 rounded-lg transition-all hover:border-[var(--accent-color)] active:scale-95" title="{$_('settings.title')}">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6 transition-transform duration-700 group-hover:rotate-180">
                    <path stroke-linecap="round" stroke-linejoin="round" d="M9.594 3.94c.09-.542.56-.94 1.11-.94h2.593c.55 0 1.02.398 1.11.94l.213 1.281c.063.374.313.686.645.87.074.04.147.083.22.127.324.196.72.257 1.075.124l1.217-.456a1.125 1.125 0 011.37.49l1.296 2.247a1.125 1.125 0 01-.26 1.431l-1.003.827c-.293.24-.438.613-.431.992a6.759 6.759 0 010 .255c-.007.378.138.75.43.99l1.005.828c.424.35.534.954.26 1.43l-1.298 2.247a1.125 1.125 0 01-1.369.491l-1.217-.456c-.355-.133-.75-.072-1.076.124a6.57 6.57 0 01-.22.128c-.331.183-.581.495-.644.869l-.213 1.28c-.09.543-.56.941-1.11.941h-2.594c-.55 0-1.02-.398-1.11-.94l-.213-1.281c-.062-.374-.312-.686-.644-.87a6.52 6.52 0 01-.22-.127c-.325-.196-.72-.257-1.076-.124l-1.217.456a1.125 1.125 0 01-1.369-.49l-1.297-2.247a1.125 1.125 0 01.26-1.431l1.004-.827c.292-.24.437-.613.43-.992a6.932 6.932 0 010-.255c.007-.378-.138-.75-.43-.99l-1.004-.828a1.125 1.125 0 01-.26-1.43l1.297-2.247a1.125 1.125 0 011.37-.491l1.216.456c.356.133.751.072 1.076-.124.072-.044.146-.087.22-.128.332-.183.582-.495.644-.869l.214-1.281z" />
                    <path stroke-linecap="round" stroke-linejoin="round" d="M15 12a3 3 0 11-6 0 3 3 0 016 0z" />
                </svg>
            </button>

            <button on:click={pickRandomGame} 
                class="bg-[var(--card-bg)] hover:bg-[#333] px-3 py-2 rounded-lg font-bold flex items-center shadow-lg transition-all border active:scale-95" 
                style="color: var(--accent-color); border-color: color-mix(in srgb, var(--accent-color), transparent 70%);"
                title="{$_('header.random_game')}">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-6 h-6"><path stroke-linecap="round" stroke-linejoin="round" d="M19.5 12c0-1.232-.046-2.453-.138-3.662a4.006 4.006 0 00-3.7-3.7 48.678 48.678 0 00-7.324 0 4.006 4.006 0 00-3.7 3.7c-.017.22-.032.441-.046.662M19.5 12l3-3m-3 3l-3-3m-12 3c0 1.232.046 2.453.138 3.662a4.006 4.006 0 003.7 3.7 48.656 48.656 0 007.324 0 4.006 4.006 0 003.7-3.7c.017-.22.032-.441.046-.662M4.5 12l3 3m-3-3l-3 3" /></svg>
            </button>

            <button on:click={() => showAddGameModal = true} class="bg-[var(--card-bg)] hover:bg-[#333] text-[var(--text-primary)] border border-white/10 px-4 py-2 rounded-lg font-bold flex items-center shadow-lg transition-all hover:border-white/30 active:scale-95">
                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5 mr-2"><path d="M10.75 4.75a.75.75 0 00-1.5 0v4.5h-4.5a.75.75 0 000 1.5h4.5v4.5a.75.75 0 001.5 0v-4.5h4.5a.75.75 0 000-1.5h-4.5v-4.5z" /></svg> {$_('header.add_game')}
            </button>

            <div class="flex items-center space-x-1 bg-[var(--card-bg)] p-1 rounded-lg border border-white/10 shadow-sm">
                <button aria-label="{$_('header.view_grid')}" on:click={() => viewMode = 'grid'} class={`p-2 rounded-md transition-all ${viewMode === 'grid' ? 'bg-[#333] text-white shadow-sm' : 'text-gray-500 hover:text-white'}`}>
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24" fill="currentColor" class="w-5 h-5"><path fill-rule="evenodd" d="M3 6a3 3 0 013-3h2.25a3 3 0 013 3v2.25a3 3 0 01-3 3H6a3 3 0 01-3-3V6zm9.75 0a3 3 0 013-3H18a3 3 0 013 3v2.25a3 3 0 01-3 3h-2.25a3 3 0 01-3-3V6zM3 15.75a3 3 0 013-3h2.25a3 3 0 013 3V18a3 3 0 01-3 3H6a3 3 0 01-3-3v-2.25zm9.75 0a3 3 0 013-3H18a3 3 0 013 3V18a3 3 0 01-3 3h-2.25a3 3 0 01-3-3v-2.25z" clip-rule="evenodd" /></svg>
                </button>
                <button aria-label="{$_('header.view_list')}" on:click={() => viewMode = 'list'} class={`p-2 rounded-md transition-all ${viewMode === 'list' ? 'bg-[#333] text-white shadow-sm' : 'text-gray-500 hover:text-white'}`}><svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" d="M3.75 12h16.5m-16.5 3.75h16.5M3.75 19.5h16.5M5.625 4.5h12.75a1.875 1.875 0 010 3.75H5.625a1.875 1.875 0 010-3.75z" /></svg></button>
                <div class="w-px h-5 bg-white/10 mx-1"></div>
                <button aria-label="Tri" on:click={cycleSort} class="flex items-center gap-2 px-3 py-2 text-xs font-bold text-gray-400 hover:text-white hover:bg-[#333] rounded-md transition-all uppercase min-w-[4rem] justify-between">
                    <span>{sortOrder === 'asc' ? $_('header.sort_az') : sortOrder === 'desc' ? $_('header.sort_za') : $_('header.sort_recent')}</span>
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class={`w-4 h-4 transition-transform duration-300 ${sortOrder === 'desc' ? 'rotate-180' : ''}`}><path stroke-linecap="round" stroke-linejoin="round" d="M3 4.5h14.25M3 9h9.75M3 13.5h9.75m4.5-4.5v12m0 0l-3.75-3.75M17.25 21L21 17.25" /></svg>
                </button>
            </div>
        </div>
    </div>

    <div class="flex items-center gap-4">
      <div class="flex-1 min-w-0 flex space-x-3 overflow-x-auto py-1 pb-2 custom-scroll">
        {#each tabs as platform}
            {@const style = getTabStyle(platform, selectedPlatform === platform)}
            {@const conf = getPlatformConfig(platform)}
            {@const count = platform === 'all' ? visibleGames.length : platform === 'fav' ? favCount : platform === 'hidden' ? hiddenCount : platformCounts[platform]}
            <button on:click={() => selectedPlatform = platform} class={`px-5 py-2 rounded-full text-sm transition-all flex items-center shrink-0 ${style}`}>
                {#if platform === 'fav'}
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class={`w-4 h-4 mr-2 ${selectedPlatform === platform ? 'text-black' : 'text-yellow-400'}`}><path fill-rule="evenodd" d="M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401z" clip-rule="evenodd" /></svg>
                    {$_('tabs.fav')}
                {:else if platform === 'all'}
                    {$_('tabs.all')}
                {:else if platform === 'hidden'}
                    <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4 mr-2"><path stroke-linecap="round" stroke-linejoin="round" d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" /></svg>
                    {$_('tabs.hidden')}
                {:else if platform === 'Custom'}
                    <span class={`w-2 h-2 rounded-full mr-2 block shadow-sm ${selectedPlatform === platform ? 'bg-black' : conf.color}`}></span>
                    {$_('tabs.custom')}
                {:else}
                    <span class={`w-2 h-2 rounded-full mr-2 block shadow-sm ${selectedPlatform === platform ? 'bg-black' : conf.color}`}></span>
                    {platform.toUpperCase()}
                {/if}
                <span class={`ml-2 text-[10px] font-bold px-1.5 rounded-full ${selectedPlatform === platform ? 'bg-black/20' : 'bg-white/10'}`}>{count || 0}</span>
            </button>
        {/each}
      </div>

      <div class="relative w-72 shrink-0">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-4 h-4 absolute left-3 top-1/2 -translate-y-1/2 text-gray-500 pointer-events-none"><path stroke-linecap="round" stroke-linejoin="round" d="M21 21l-5.197-5.197m0 0A7.5 7.5 0 105.196 5.196a7.5 7.5 0 0010.607 10.607z" /></svg>
          <input bind:this={searchInput} bind:value={searchQuery} type="search" placeholder={$_('header.search')}
              class="w-full bg-[var(--card-bg)] text-[var(--text-primary)] text-sm pl-9 pr-14 py-2.5 rounded-full border border-white/10 focus:border-[var(--accent-color)] outline-none transition-colors placeholder:text-gray-500" />
          {#if !searchQuery}
              <kbd class="absolute right-3 top-1/2 -translate-y-1/2 text-[10px] text-gray-500 border border-white/10 rounded px-1.5 py-0.5 pointer-events-none">Ctrl F</kbd>
          {/if}
      </div>

      <button on:click={refreshLibrary} disabled={isScanning} title={$_('header.refresh')} aria-label={$_('header.refresh')}
          class="shrink-0 bg-[var(--card-bg)] text-gray-400 hover:text-[var(--accent-color)] border border-white/10 hover:border-[var(--accent-color)] p-2.5 rounded-full transition-all active:scale-95 disabled:opacity-60">
          <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class={`w-4 h-4 ${isScanning ? 'animate-spin' : ''}`}><path stroke-linecap="round" stroke-linejoin="round" d="M16.023 9.348h4.992v-.001M2.985 19.644v-4.992m0 0h4.992m-4.993 0l3.181 3.183a8.25 8.25 0 0013.803-3.7M4.031 9.865a8.25 8.25 0 0113.803-3.7l3.181 3.182m0-4.991v4.99" /></svg>
      </button>
    </div>
  </header>
  
  {#if loadingError} <div class="p-4 bg-red-900/20 border border-red-500/50 rounded-lg text-red-200 mb-4 backdrop-blur-sm">{$_('error')}: {loadingError}</div> {/if}

  <div class="flex-1 overflow-y-auto pr-2 pb-6 custom-scroll z-0">
    {#if showRecentRow}
        <section class="mb-8 animate-fade-in">
            <h2 class="text-xs font-black uppercase tracking-[0.2em] text-gray-400 mb-3">{$_('recent.title')}</h2>
            <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(260px,1fr))]">
                {#each recentGames as game (game.platform + game.id)}
                {@const wide = getWideSrc(game, brokenImgs, onlineArt, steamArt)}
                <div on:click={() => openDetails(game)} class="group relative aspect-[616/353] rounded-xl overflow-hidden bg-[#141414] border border-white/5 cursor-pointer hover:-translate-y-1 hover:shadow-[0_10px_30px_-10px_rgba(0,0,0,0.7)] transition-all duration-300">
                    {#if wide}
                        <img src={wide} alt={game.title} on:error={() => markBroken(wide)} class="absolute inset-0 w-full h-full object-cover transition-transform duration-700 group-hover:scale-105" />
                    {/if}
                    <div class="absolute inset-0 bg-gradient-to-t from-black/95 via-black/30 to-transparent"></div>
                    <div class="absolute bottom-0 left-0 right-0 p-3 flex items-end justify-between gap-2">
                        <div class="min-w-0">
                            <h3 class="font-bold text-white text-sm truncate drop-shadow">{game.title}</h3>
                            <p class="text-[11px] text-gray-300">{timeAgo(game.last_played, $locale)}</p>
                        </div>
                        <button aria-label={$_('game_card.play')} on:click|stopPropagation={() => play(game)} class="shrink-0 bg-white text-black p-2.5 rounded-full shadow-lg opacity-90 group-hover:opacity-100 hover:scale-110 transition-all">
                            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z" /></svg>
                        </button>
                    </div>
                </div>
                {/each}
            </div>
            <h2 class="text-xs font-black uppercase tracking-[0.2em] text-gray-400 mt-8">{$_('recent.library')}</h2>
        </section>
    {/if}

    {#if isScanning && games.length === 0}
        <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(180px,1fr))]">
            {#each Array(12) as _skeleton}
                <div class="h-80 rounded-xl bg-[var(--card-bg)] border border-white/5 animate-pulse"></div>
            {/each}
        </div>
    {:else if processedGames.length === 0}
        <div class="h-full flex flex-col items-center justify-center text-center text-gray-500 gap-3 animate-fade-in">
            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1" stroke="currentColor" class="w-16 h-16 opacity-40"><path stroke-linecap="round" stroke-linejoin="round" d="M14.25 6.087c0-.355.186-.676.401-.959.221-.29.349-.634.349-1.003 0-1.036-1.007-1.875-2.25-1.875s-2.25.84-2.25 1.875c0 .369.128.713.349 1.003.215.283.401.604.401.959v0a.64.64 0 01-.657.643 48.39 48.39 0 01-4.163-.3c.186 1.613.293 3.25.315 4.907a.656.656 0 01-.658.663v0c-.355 0-.676-.186-.959-.401a1.647 1.647 0 00-1.003-.349c-1.036 0-1.875 1.007-1.875 2.25s.84 2.25 1.875 2.25c.369 0 .713-.128 1.003-.349.283-.215.604-.401.959-.401v0c.31 0 .555.26.532.57a48.039 48.039 0 01-.642 5.056c1.518.19 3.058.309 4.616.354a.64.64 0 00.657-.643v0c0-.355-.186-.676-.401-.959a1.647 1.647 0 01-.349-1.003c0-1.035 1.008-1.875 2.25-1.875 1.243 0 2.25.84 2.25 1.875 0 .369-.128.713-.349 1.003-.215.283-.4.604-.4.959v0c0 .333.277.599.61.58a48.1 48.1 0 005.427-.63 48.05 48.05 0 00.582-4.717.532.532 0 00-.533-.57v0c-.355 0-.676.186-.959.401-.29.221-.634.349-1.003.349-1.035 0-1.875-1.007-1.875-2.25s.84-2.25 1.875-2.25c.37 0 .713.128 1.003.349.283.215.604.401.96.401v0a.656.656 0 00.658-.663 48.422 48.422 0 00-.37-5.36c-1.886.342-3.81.574-5.766.689a.578.578 0 01-.61-.58v0z" /></svg>
            <p class="font-medium">{searchQuery ? $_('empty.no_results') : $_('empty.no_games')}</p>
            {#if searchQuery}
                <button on:click={() => searchQuery = ""} class="text-sm text-[var(--accent-color)] hover:underline">{$_('empty.clear_search')}</button>
            {/if}
        </div>
    {:else if viewMode === 'grid'}
        <div class="grid gap-4 grid-cols-[repeat(auto-fill,minmax(180px,1fr))]">
            {#each processedGames as game, i (game.id + game.platform + i)}
            {@const conf = getPlatformConfig(game.platform)}
            {@const src = getImgSrc(game, brokenImgs, onlineArt, steamArt)}

            <div on:click={() => openDetails(game)} class={`group relative bg-[var(--card-bg)] rounded-xl overflow-hidden hover:-translate-y-1 transition-all duration-300 hover:shadow-[0_10px_20px_-10px_rgba(0,0,0,0.5)] border hover:border-opacity-50 flex flex-col h-80 animate-fade-in cursor-default border-white/5`}>
                
                <button aria-label="{$_('game_card.favorite')}" on:click={(e) => toggleFav(e, game)} class={`absolute top-2 right-2 z-50 p-2 rounded-full shadow-lg transition-all backdrop-blur-md ${game.is_favorite ? 'bg-white text-black' : 'bg-black/40 text-white/50 hover:bg-white hover:text-black'}`}>
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401z" clip-rule="evenodd" /></svg>
                </button>

                <div class="relative w-full h-64 bg-[#141414] overflow-hidden">
                    {#if src}
                        <img
                            {src}
                            alt={game.title}
                            on:error={() => markBroken(src)}
                            class="w-full h-full object-cover z-10 relative transition-transform duration-500 group-hover:scale-105"
                            class:fallback-zoomed={src.endsWith('header.jpg')}
                            loading="lazy"
                            decoding="async"
                        />
                    {:else}
                        <div class="absolute inset-0 flex flex-col items-center justify-center gap-2 p-4 text-center bg-gradient-to-br from-[#1f1f1f] to-[#0d0d0d]">
                            <span class="text-5xl font-black text-white/10">{game.title.charAt(0).toUpperCase()}</span>
                            <span class="text-xs font-bold text-white/30 line-clamp-2">{game.title}</span>
                        </div>
                    {/if}
                    
                    <div class="absolute inset-0 bg-black/80 opacity-0 group-hover:opacity-100 transition-all duration-300 flex flex-col items-center justify-center backdrop-blur-sm z-40 gap-3 p-4">
                        <button on:click={(e) => { e.stopPropagation(); play(game); }} class="w-full bg-white text-black font-black py-3 px-4 rounded-lg shadow-[0_0_20px_rgba(255,255,255,0.2)] flex items-center justify-center hover:scale-105 transition-transform tracking-wider"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5 mr-2"><path d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z" /></svg> {$_('game_card.play')}</button>
                        
                        <div class="flex w-full gap-2">
                            <button on:click={(e) => { e.stopPropagation(); openInLauncher(game); }} class="flex-1 bg-black/50 text-gray-200 font-semibold py-2 px-2 rounded-lg border border-white/20 transition-all duration-200 flex items-center justify-center text-xs uppercase tracking-wide truncate backdrop-blur-md hover:bg-black/90">
                                {game.platform === 'Custom' ? $_('game_card.folder') : game.platform}
                            </button>
                            <button on:click={(e) => { e.stopPropagation(); openDetails(game); }} class="bg-black/50 hover:bg-white hover:text-black text-white p-2 rounded-lg border border-white/20 transition-all backdrop-blur-md" title="{$_('game_card.details')}">
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
            {#each processedGames as game, i (game.id + game.platform + i)}
            {@const conf = getPlatformConfig(game.platform)}
            {@const src = getImgSrc(game, brokenImgs, onlineArt, steamArt)}
            <div on:click={() => openDetails(game)} class="group flex items-center bg-[var(--card-bg)] hover:brightness-110 rounded-xl p-2 pr-4 transition-all border border-white/5 cursor-pointer animate-fade-in hover:shadow-md">
                <button aria-label="{$_('game_card.favorite')}" on:click={(e) => toggleFav(e, game)} class={`mr-3 p-2 rounded-full transition-all ${game.is_favorite ? 'text-white bg-white/10' : 'text-gray-600 hover:text-white hover:bg-white/5'}`}>
                    <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4"><path fill-rule="evenodd" d="M10.868 2.884c-.321-.772-1.415-.772-1.736 0l-1.83 4.401-4.753.381c-.833.067-1.171 1.107-.536 1.651l3.62 3.102-1.106 4.637c-.194.813.691 1.456 1.405 1.02L10 15.591l4.069 2.485c.713.436 1.598-.207 1.404-1.02l-1.106-4.637 3.62-3.102c.635-.544.297-1.584-.536-1.65l-4.752-.382-1.831-4.401z" clip-rule="evenodd" /></svg>
                </button>
                <div class="w-12 h-16 bg-[#141414] rounded-lg overflow-hidden shrink-0 relative mr-4 border border-white/5">
                    
                    {#if src}
                        <img
                            {src}
                            alt=""
                            on:error={() => markBroken(src)}
                            class="w-full h-full object-cover"
                            class:fallback-zoomed={src.endsWith('header.jpg')}
                            loading="lazy"
                            decoding="async"
                        />
                    {:else}
                        <div class="w-full h-full flex items-center justify-center text-xl font-black text-white/15">{game.title.charAt(0).toUpperCase()}</div>
                    {/if}

                </div>
                <div class="flex-1 min-w-0">
                    <h3 class="font-bold text-[var(--text-primary)] text-sm truncate mb-1">{game.title}</h3>
                    <div class="flex items-center"><span class={`w-1.5 h-1.5 rounded-full mr-2 ${conf.color}`}></span><span class={`text-xs uppercase font-bold ${conf.text}`}>{game.platform}</span></div>
                </div>
                {#if game.last_played}
                    <span class="text-xs text-gray-500 mr-4 hidden md:block">{timeAgo(game.last_played, $locale)}</span>
                {/if}
                <div class="flex items-center opacity-0 group-hover:opacity-100 transition-all space-x-2">
                    <button aria-label="{$_('game_card.play')}" on:click={(e) => { e.stopPropagation(); play(game); }} class="bg-white text-black p-2 rounded-full shadow-[0_0_10px_rgba(255,255,255,0.3)] transition-all hover:scale-110 hover:shadow-[0_0_15px_rgba(255,255,255,0.5)] ml-2"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5"><path d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z" /></svg></button>
                </div>
            </div>
            {/each}
        </div>
    {/if}
  </div>

  {#if showAddGameModal}
    <div class="fixed inset-0 z-[150] flex items-center justify-center p-4 bg-black/90 backdrop-blur-sm animate-fade-in">
        <div class="bg-[var(--card-bg)] w-full max-w-md p-6 rounded-2xl shadow-[0_0_30px_rgba(0,0,0,0.5)] border border-white/10">
            <h2 class="text-2xl font-black text-[var(--text-primary)] mb-6 tracking-wide">{$_('modal_add.title')}</h2>
            <div class="space-y-5">
                <label class="block">
                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider ml-1">{$_('modal_add.label_title')}</span>
                    <input type="text" bind:value={newGame.title} class="mt-2 w-full bg-black/20 text-[var(--text-primary)] p-3 rounded-xl border border-white/10 focus:border-white/50 outline-none transition-colors font-medium placeholder:text-gray-600" placeholder="{$_('modal_add.placeholder_title')}" />
                </label>
                <div>
                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider ml-1">{$_('modal_add.label_exe')}</span>
                    <div class="flex gap-2 mt-2 group">
                        <input type="text" value={newGame.exePath} readonly class="flex-1 bg-black/20 text-gray-500 p-3 rounded-xl border border-white/10 text-sm group-hover:border-white/30 transition-colors truncate" placeholder="{$_('modal_add.placeholder_path')}" />
                        <button on:click={pickExe} class="bg-[#333] hover:bg-white hover:text-black text-white px-4 rounded-xl border border-white/10 transition-all flex items-center justify-center">
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 12.75V12A2.25 2.25 0 014.5 9.75h15A2.25 2.25 0 0121.75 12v.75m-8.69-6.44l-2.12-2.12a1.5 1.5 0 00-1.061-.44H4.5A2.25 2.25 0 002.25 6v12a2.25 2.25 0 002.25 2.25h15A2.25 2.25 0 0021.75 18V9a2.25 2.25 0 00-2.25-2.25h-5.379a1.5 1.5 0 01-1.06-.44z" /></svg>
                        </button>
                    </div>
                </div>
                <div>
                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider ml-1">{$_('modal_add.label_img')}</span>
                    <div class="flex gap-2 mt-2 group">
                        <input type="text" value={newGame.imgPath} readonly class="flex-1 bg-black/20 text-gray-500 p-3 rounded-xl border border-white/10 text-sm group-hover:border-white/30 transition-colors truncate" placeholder="{$_('modal_add.placeholder_img')}" />
                        <button on:click={pickImage} class="bg-[#333] hover:bg-white hover:text-black text-white px-4 rounded-xl border border-white/10 transition-all flex items-center justify-center">
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-6 h-6"><path stroke-linecap="round" stroke-linejoin="round" d="M2.25 15.75l5.159-5.159a2.25 2.25 0 013.182 0l5.159 5.159m-1.5-1.5l1.409-1.409a2.25 2.25 0 013.182 0l2.909 2.909m-18 3.75h16.5a1.5 1.5 0 001.5-1.5V6a1.5 1.5 0 00-1.5-1.5H3.75A1.5 1.5 0 002.25 6v12a1.5 1.5 0 001.5 1.5zm10.5-11.25h.008v.008h-.008V8.25zm.375 0a.375.375 0 11-.75 0 .375.375 0 01.75 0z" /></svg>
                        </button>
                    </div>
                </div>
            </div>
            <div class="flex justify-end gap-3 mt-8">
                <button on:click={() => showAddGameModal = false} class="text-gray-400 hover:text-white px-4 py-2 font-bold transition-colors uppercase text-sm tracking-wider">{$_('modal_add.cancel')}</button>
                <button on:click={saveCustomGame} class="bg-white text-black hover:bg-gray-200 px-6 py-2 rounded-xl font-black shadow-lg transition-all hover:scale-105 hover:shadow-white/20 uppercase text-sm tracking-wider flex items-center"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-4 h-4 mr-2"><path d="M10.75 4.75a.75.75 0 00-1.5 0v4.5h-4.5a.75.75 0 000 1.5h4.5v4.5a.75.75 0 001.5 0v-4.5h4.5a.75.75 0 000-1.5h-4.5v-4.5z" /></svg> {$_('modal_add.confirm')}</button>
            </div>
        </div>
    </div>
  {/if}

  {#if showSettingsModal}
    <div class="fixed inset-0 z-[150] flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fade-in">
        <div class="bg-[var(--card-bg)] w-full max-w-4xl h-[70vh] rounded-2xl shadow-[0_0_50px_rgba(0,0,0,0.6)] border border-white/10 flex overflow-hidden">
            
            <div class="w-64 bg-black/20 border-r border-white/5 p-6 flex flex-col gap-2">
                <h2 class="text-xl font-black text-[var(--text-primary)] mb-6 tracking-wide uppercase px-2">{$_('settings.title')}</h2>
                <button on:click={() => settingsTab = 'general'} class={`text-left px-4 py-3 rounded-lg font-bold transition-all ${settingsTab === 'general' ? 'bg-[var(--accent-color)] text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}>{$_('settings.tabs.general')}</button>
                <button on:click={() => settingsTab = 'appearance'} class={`text-left px-4 py-3 rounded-lg font-bold transition-all ${settingsTab === 'appearance' ? 'bg-[var(--accent-color)] text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}>{$_('settings.tabs.appearance')}</button>
                <button on:click={() => settingsTab = 'about'} class={`text-left px-4 py-3 rounded-lg font-bold transition-all ${settingsTab === 'about' ? 'bg-[var(--accent-color)] text-white shadow-lg' : 'text-gray-400 hover:text-white hover:bg-white/5'}`}>{$_('settings.tabs.about')}</button>
                
                <div class="flex-1"></div>
                <button on:click={closeSettings} class="text-left px-4 py-3 rounded-lg font-bold text-gray-500 hover:text-white transition-colors">{$_('settings.close')}</button>
            </div>

            <div class="flex-1 p-8 overflow-y-auto custom-scroll bg-[var(--card-bg)]">
                
                {#if settingsTab === 'general'}
                    <div class="space-y-6 animate-fade-in">
                        
                        <div class="flex items-center justify-between p-4 bg-black/20 rounded-xl border border-white/5 mb-6">
                            <div>
                                <h4 class="font-bold text-[var(--text-primary)]">{$_('settings.general.autostart_title')}</h4>
                                <p class="text-xs text-gray-500">{$_('settings.general.autostart_desc')}</p>
                            </div>
                            <button on:click={toggleAutostart} class={`w-12 h-6 rounded-full transition-colors relative ${autostartEnabled ? 'bg-[var(--accent-color)]' : 'bg-gray-700'}`}>
                                <span class={`absolute top-1 left-1 bg-white w-4 h-4 rounded-full transition-transform ${autostartEnabled ? 'translate-x-6' : ''}`}></span>
                            </button>
                        </div>

                        <h3 class="text-2xl font-bold text-[var(--text-primary)] mb-4">{$_('settings.general.drives_title')}</h3>
                        <p class="text-gray-400 text-sm mb-4">{$_('settings.general.drives_desc')}</p>
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
                                            {$_('settings.general.drive')} {drive}
                                    </span>
                                </label>
                            {/each}
                        </div>
                        {#if availableDrives.length === 0}
                            <p class="text-sm text-yellow-400 mt-2 bg-yellow-400/10 p-3 rounded-lg border border-yellow-400/20">{$_('settings.general.no_drives')}</p>
                        {/if}
                    </div>

                {:else if settingsTab === 'appearance'}
                    <div class="space-y-8 animate-fade-in">
                        <div>
                            <h3 class="text-2xl font-bold text-[var(--text-primary)] mb-4">{$_('settings.appearance.presets_title')}</h3>
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
                            <h3 class="text-2xl font-bold text-[var(--text-primary)] mb-6">{$_('settings.appearance.custom_title')}</h3>
                            
                            <div class="grid grid-cols-1 md:grid-cols-2 gap-6">
                                <label class="block space-y-2">
                                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">{$_('settings.appearance.accent')}</span>
                                    <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                        <input type="color" bind:value={currentTheme.accent} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/>
                                        <span class="text-gray-300 font-mono text-sm">{currentTheme.accent}</span>
                                    </div>
                                </label>

                                <label class="block space-y-2">
                                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">{$_('settings.appearance.card_bg')}</span>
                                    <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                        <input type="color" bind:value={currentTheme.card_bg} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/>
                                        <span class="text-gray-300 font-mono text-sm">{currentTheme.card_bg}</span>
                                    </div>
                                </label>

                                <label class="block space-y-2">
                                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">{$_('settings.appearance.bg_top')}</span>
                                    <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                        <input type="color" bind:value={currentTheme.bg_from} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/>
                                        <span class="text-gray-300 font-mono text-sm">{currentTheme.bg_from}</span>
                                    </div>
                                </label>

                                <label class="block space-y-2">
                                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">{$_('settings.appearance.bg_bottom')}</span>
                                    <div class="flex gap-3 items-center bg-black/20 p-2 rounded-lg border border-white/5">
                                        <input type="color" bind:value={currentTheme.bg_to} on:input={applyTheme} class="w-10 h-10 rounded cursor-pointer bg-transparent border-none"/>
                                        <span class="text-gray-300 font-mono text-sm">{currentTheme.bg_to}</span>
                                    </div>
                                </label>
                                
                                <label class="block space-y-2">
                                    <span class="text-gray-400 text-xs uppercase font-bold tracking-wider">{$_('settings.appearance.text_main')}</span>
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
                            <p class="text-gray-400 mt-2">{$_('settings.about.version')} {appVersion}</p>
                        </div>
                        
                        <div class="flex flex-col gap-3 w-full max-w-sm mt-8">
                            <button 
                                on:click={() => invoke('open_launcher_page', { id: 'mailto:s99ddhhh9@proton.me?subject=Feedback%20GameHub', platform: 'Custom' })}
                                class="bg-[#333] hover:bg-white hover:text-black text-white px-6 py-4 rounded-xl font-bold shadow-lg transition-all flex items-center justify-center"
                            >
                                <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-5 h-5 mr-3"><path fill-rule="evenodd" d="M2.25 2.25A2.25 2.25 0 000 4.5v11A2.25 2.25 0 002.25 18h15.5A2.25 2.25 0 0020 15.5v-11A2.25 2.25 0 0017.75 2.25H2.25zM17.5 5.5v1.75a.75.75 0 01-1.5 0V5.5H4v1.75a.75.75 0 01-1.5 0V5.5H2.25c-.138 0-.25.112-.25.25V15.5c0 .138.112.25.25.25h15.5c.138 0 .25-.112.25-.25V5.75c0-.138-.112-.25-.25-.25H17.5zM6.5 10a1.5 1.5 0 011.5 1.5v2a1.5 1.5 0 01-3 0v-2A1.5 1.5 0 016.5 10zM13.5 10a1.5 1.5 0 011.5 1.5v2a1.5 1.5 0 01-3 0v-2A1.5 1.5 0 0113.5 10z" clip-rule="evenodd" /></svg>
                                {$_('settings.about.report_btn')}
                            </button>
                            <button 
                                on:click={() => openLink('https://buymeacoffee.com/geewerrr')}
                                class="bg-[#FFDD00] text-black font-black px-6 py-4 rounded-xl shadow-lg hover:scale-105 transition-all flex items-center justify-center"
                            >
                                <svg class="w-5 h-5 mr-3" fill="currentColor" viewBox="0 0 24 24" xmlns="http://www.w3.org/2000/svg"><path d="M20.216 6.415l-.132-.666c-.119-.596-.385-1.135-.77-1.566-.751-.841-1.869-1.328-3.006-1.298-2.56.069-12.068 1.583-12.068 1.583-.759.128-1.344.693-1.483 1.445L1.95 10.39c-.141.757.252 1.496.954 1.785a1.997 1.997 0 0 0 2.555-.658l.183-.293c.336 2.543 2.421 4.536 5.007 4.794v2.484H7.917a1 1 0 1 0 0 2h8.166a1 1 0 1 0 0-2h-2.732v-2.496c2.552-.28 4.609-2.254 4.954-4.763l.679.152a2.004 2.004 0 0 0 2.358-1.221 2.003 2.003 0 0 0-1.126-2.463zM16.57 10.202l-.476 2.143c-.219.984-1.157 1.622-2.145 1.458-4.336-.719-7.466-3.548-6.99-5.688.075-.337.254-.639.51-.875 1.324-1.214 7.185-1.494 8.503-1.276.989.164 1.66 1.077 1.441 2.062l-.843 2.176z"/></svg>
                                {$_('settings.about.coffee_btn')}
                            </button>
                        </div>
                    </div>
                {/if}

            </div>
            
            <div class="absolute bottom-6 right-8">
                <button on:click={saveSettings} class="bg-[var(--accent-color)] text-white hover:brightness-110 px-6 py-3 rounded-xl font-black shadow-lg shadow-[var(--accent-color)]/20 transition-all hover:scale-105 uppercase text-sm tracking-wider">
                    {$_('settings.save')}
                </button>
            </div>
        </div>
    </div>
  {/if}

  {#if selectedGame}
    {@const hero = getHeroSrc(selectedGame, gameDetails, brokenImgs, onlineArt, steamArt)}
    {@const pconf = getPlatformConfig(selectedGame.platform)}
    <div class="fixed inset-0 z-[100] flex items-center justify-center p-8 animate-fade-in">
        <div class="absolute inset-0 bg-black/90 backdrop-blur-md" on:click={closeDetails}></div>
        <div class="bg-[var(--card-bg)] w-full max-w-6xl h-full max-h-[88vh] rounded-2xl shadow-[0_0_60px_rgba(0,0,0,0.8)] relative flex flex-col overflow-hidden ring-1 ring-white/10">
            <button aria-label="Fermer" on:click={closeDetails} class="absolute top-4 right-4 z-50 bg-black/50 text-white p-2 rounded-full hover:bg-white hover:text-black transition-colors backdrop-blur-md border border-white/10">
                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="2" stroke="currentColor" class="w-5 h-5"><path stroke-linecap="round" stroke-linejoin="round" d="M6 18L18 6M6 6l12 12" /></svg>
            </button>

            <!-- Bannière HD (library_hero 3840x1240 quand Steam la fournit) -->
            <div class="relative h-[40%] min-h-[220px] shrink-0 bg-[#0d0d0d] overflow-hidden">
                {#if hero}
                    <img src={hero} alt="" on:error={() => markBroken(hero)} class="absolute inset-0 w-full h-full object-cover hero-zoom" />
                {/if}
                <div class="absolute inset-0" style="background: linear-gradient(to top, var(--card-bg) 0%, color-mix(in srgb, var(--card-bg), transparent 40%) 35%, transparent 75%);"></div>
                <div class="absolute bottom-0 left-0 right-0 px-10 pb-6 flex items-end justify-between gap-6">
                    <div class="min-w-0">
                        <h1 class="text-5xl font-black text-white tracking-wide uppercase drop-shadow-[0_4px_20px_rgba(0,0,0,0.9)] truncate" title={selectedGame.title}>{selectedGame.title}</h1>
                        <div class="flex items-center gap-4 mt-2 text-xs font-bold uppercase tracking-widest">
                            <span class="flex items-center"><span class={`w-1.5 h-1.5 rounded-full mr-2 ${pconf.color}`}></span><span class={pconf.text}>{selectedGame.platform}</span></span>
                            {#if selectedGame.last_played}
                                <span class="text-gray-300">{$_('details.last_played')} {timeAgo(selectedGame.last_played, $locale)}</span>
                            {/if}
                        </div>
                    </div>
                    <div class="flex gap-3 shrink-0">
                        <button on:click={() => play(selectedGame)} class="bg-white text-black hover:bg-gray-200 py-4 px-10 rounded-xl font-black shadow-[0_0_30px_rgba(255,255,255,0.25)] flex items-center justify-center text-lg hover:scale-[1.03] transition-all tracking-wider uppercase">
                            <svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 20 20" fill="currentColor" class="w-6 h-6 mr-2"><path d="M6.3 2.841A1.5 1.5 0 004 4.11V15.89a1.5 1.5 0 002.3 1.269l9.344-5.89a1.5 1.5 0 000-2.538L6.3 2.84z" /></svg> {$_('details.play_btn')}
                        </button>
                        <button on:click={() => openInLauncher(selectedGame)} class="bg-black/50 backdrop-blur-md hover:bg-white hover:text-black text-white px-6 py-4 rounded-xl border border-white/20 font-bold flex items-center transition-all uppercase tracking-wider text-sm">
                            {selectedGame.platform === 'Custom' ? $_('details.folder_btn') : $_('details.open_btn')}
                        </button>
                    </div>
                </div>
            </div>

            <div class="flex-1 min-h-0 flex">
                <!-- Colonne infos -->
                <div class="w-80 shrink-0 p-8 pt-4 overflow-y-auto custom-scroll border-r border-white/5 space-y-6">
                    {#if gameDetails}
                        <div class="space-y-3">
                            <div class="flex justify-between gap-4 text-sm border-b border-white/10 pb-3">
                                <span class="text-gray-500 font-medium uppercase tracking-wider text-xs">{$_('details.developer')}</span>
                                <span class="text-[var(--text-primary)] font-bold text-right">{gameDetails.developers ? gameDetails.developers[0] : $_('details.unknown')}</span>
                            </div>
                            <div class="flex justify-between gap-4 text-sm border-b border-white/10 pb-3">
                                <span class="text-gray-500 font-medium uppercase tracking-wider text-xs">{$_('details.date')}</span>
                                <span class="text-[var(--text-primary)] font-bold text-right">{gameDetails.release_date?.date || $_('details.unknown')}</span>
                            </div>
                            {#if gameDetails.metacritic}
                            <div class="flex justify-between items-center text-sm border-b border-white/10 pb-3">
                                <span class="text-gray-500 font-medium uppercase tracking-wider text-xs">{$_('details.metacritic')}</span>
                                <span class={`font-black px-3 py-1 rounded text-xs ${gameDetails.metacritic.score >= 75 ? 'bg-green-500 text-black' : gameDetails.metacritic.score >= 50 ? 'bg-yellow-400 text-black' : 'bg-red-500 text-white'}`}>{gameDetails.metacritic.score}</span>
                            </div>
                            {/if}
                        </div>
                        {#if gameDetails.genres?.length}
                            <div class="flex flex-wrap gap-2">
                                {#each gameDetails.genres.slice(0, 6) as genre}
                                    <span class="text-[11px] font-bold px-2.5 py-1 rounded-full bg-white/5 border border-white/10 text-gray-300">{genre.description}</span>
                                {/each}
                            </div>
                        {/if}
                    {/if}

                    {#if gameHltb}
                        <div class="animate-fade-in">
                            <div class="flex items-center mb-3">
                                <span class="text-gray-500 font-black uppercase tracking-widest text-[10px] mr-2">{$_('details.hltb_title')}</span>
                                <span class="text-[10px] bg-black/30 text-gray-400 px-1 rounded border border-white/5">HLTB</span>
                            </div>
                            <div class="space-y-2">
                                <div class="flex justify-between items-center bg-black/30 p-2 rounded border border-white/5">
                                    <span class="text-gray-400 text-xs">{$_('details.hltb_story')}</span>
                                    <span class="text-[var(--text-primary)] font-bold text-sm">{gameHltb.main}h</span>
                                </div>
                                <div class="flex justify-between items-center bg-black/30 p-2 rounded border border-white/5">
                                    <span class="text-gray-400 text-xs">{$_('details.hltb_extra')}</span>
                                    <span class="text-[var(--accent-color)] font-bold text-sm">{gameHltb.main_extra}h</span>
                                </div>
                                <div class="flex justify-between items-center bg-black/30 p-2 rounded border border-white/5">
                                    <span class="text-gray-400 text-xs">{$_('details.hltb_100')}</span>
                                    <span class="text-yellow-500 font-bold text-sm">{gameHltb.completionist}h</span>
                                </div>
                            </div>
                        </div>
                    {/if}

                    {#if gameMods}
                        <div class="animate-fade-in">
                            <span class="text-gray-500 font-black uppercase tracking-widest text-[10px] block mb-3">{$_('details.mods_title')}</span>
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

                    <div class="pt-4 border-t border-white/10 flex flex-col gap-2">
                        <button on:click={() => toggleHidden(selectedGame)} class="w-full text-left text-sm font-bold text-gray-400 hover:text-[var(--text-primary)] px-3 py-2 rounded-lg hover:bg-white/5 transition-colors flex items-center gap-2">
                            <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M3.98 8.223A10.477 10.477 0 001.934 12C3.226 16.338 7.244 19.5 12 19.5c.993 0 1.953-.138 2.863-.395M6.228 6.228A10.45 10.45 0 0112 4.5c4.756 0 8.773 3.162 10.065 7.498a10.523 10.523 0 01-4.293 5.774M6.228 6.228L3 3m3.228 3.228l3.65 3.65m7.894 7.894L21 21m-3.228-3.228l-3.65-3.65m0 0a3 3 0 10-4.243-4.243m4.242 4.242L9.88 9.88" /></svg>
                            {selectedGame.is_hidden ? $_('details.unhide') : $_('details.hide')}
                        </button>
                        {#if selectedGame.platform === 'Custom'}
                            <button on:click={() => removeCustomGame(selectedGame)} class="w-full text-left text-sm font-bold text-red-400/80 hover:text-red-400 px-3 py-2 rounded-lg hover:bg-red-500/10 transition-colors flex items-center gap-2">
                                <svg xmlns="http://www.w3.org/2000/svg" fill="none" viewBox="0 0 24 24" stroke-width="1.5" stroke="currentColor" class="w-4 h-4"><path stroke-linecap="round" stroke-linejoin="round" d="M14.74 9l-.346 9m-4.788 0L9.26 9m9.968-3.21c.342.052.682.107 1.022.166m-1.022-.165L18.16 19.673a2.25 2.25 0 01-2.244 2.077H8.084a2.25 2.25 0 01-2.244-2.077L4.772 5.79m14.456 0a48.108 48.108 0 00-3.478-.397m-12 .562c.34-.059.68-.114 1.022-.165m0 0a48.11 48.11 0 013.478-.397m7.5 0v-.916c0-1.18-.91-2.164-2.09-2.201a51.964 51.964 0 00-3.32 0c-1.18.037-2.09 1.022-2.09 2.201v.916m7.5 0a48.667 48.667 0 00-7.5 0" /></svg>
                                {$_('details.remove')}
                            </button>
                        {/if}
                    </div>
                </div>

                <!-- Description + captures -->
                <div class="flex-1 min-w-0 p-8 pt-4 overflow-y-auto custom-scroll">
                    {#if isLoadingDetails}
                        <div class="h-full flex items-center justify-center flex-col">
                            <div class="w-10 h-10 border-4 border-white/20 border-t-[var(--accent-color)] rounded-full animate-spin mb-4"></div>
                            <p class="text-gray-400 font-medium tracking-wider uppercase text-sm">{$_('details.loading')}</p>
                        </div>
                    {:else if gameDetails}
                        <p class="text-gray-300 leading-relaxed text-[15px] max-w-3xl">{stripHtml(gameDetails.short_description)}</p>
                        {#if gameDetails.screenshots?.length}
                            <h3 class="text-gray-500 font-black uppercase tracking-widest text-[10px] mt-8 mb-3">{$_('details.screenshots')}</h3>
                            <div class="grid grid-cols-2 xl:grid-cols-3 gap-3">
                                {#each gameDetails.screenshots.slice(0, 9) as shot (shot.id)}
                                    <button on:click={() => lightboxUrl = shot.path_full} class="group/shot relative aspect-video rounded-lg overflow-hidden ring-1 ring-white/10 hover:ring-[var(--accent-color)] transition-all">
                                        <img src={shot.path_thumbnail} alt="" loading="lazy" class="w-full h-full object-cover transition-transform duration-500 group-hover/shot:scale-110" />
                                    </button>
                                {/each}
                            </div>
                        {/if}
                    {:else}
                        <div class="h-full flex items-center justify-center">
                            <p class="text-gray-500 max-w-md text-center font-medium">{$_('details.no_info')}</p>
                        </div>
                    {/if}
                </div>
            </div>
        </div>
    </div>
  {/if}

  {#if lightboxUrl}
    <div class="fixed inset-0 z-[160] bg-black/95 flex items-center justify-center p-10 animate-fade-in cursor-zoom-out" on:click={() => lightboxUrl = null}>
        <img src={lightboxUrl} alt="" class="max-w-full max-h-full rounded-xl shadow-2xl ring-1 ring-white/10" />
    </div>
  {/if}

  {#if toastMsg}
    <div class="fixed bottom-6 left-1/2 -translate-x-1/2 z-[300] bg-[var(--card-bg)] text-[var(--text-primary)] border border-white/10 px-5 py-3 rounded-xl shadow-2xl font-bold text-sm animate-fade-in flex items-center gap-3">
        <span class="w-2 h-2 rounded-full bg-[var(--accent-color)] animate-pulse"></span>
        {toastMsg}
    </div>
  {/if}

  {#if showAutostartPopup}
    <div class="fixed inset-0 z-[200] flex items-center justify-center p-4 bg-black/80 backdrop-blur-md animate-fade-in">
        <div class="bg-[var(--card-bg)] p-6 rounded-2xl border border-white/10 max-w-sm w-full shadow-2xl">
            <h3 class="text-xl font-bold mb-2 text-[var(--text-primary)]">{$_('autostart_popup.title')}</h3>
            <p class="text-gray-400 mb-6 text-sm">{$_('autostart_popup.msg')}</p>
            <div class="flex gap-3 justify-end">
                <button on:click={() => closeAutostartPopup(true)} class="text-gray-500 text-xs hover:text-white mr-auto">{$_('autostart_popup.dont_ask')}</button>
                <button on:click={() => closeAutostartPopup(false)} class="px-4 py-2 rounded-lg bg-white/5 hover:bg-white/10 text-white font-bold transition-colors">{$_('autostart_popup.no')}</button>
                <button on:click={() => { toggleAutostart(); closeAutostartPopup(false); }} class="px-4 py-2 rounded-lg bg-[var(--accent-color)] text-white font-bold hover:brightness-110 shadow-lg transition-all">{$_('autostart_popup.yes')}</button>
            </div>
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
  
  .hero-zoom { animation: heroZoom 14s ease-out forwards; }
  @keyframes heroZoom { from { transform: scale(1.08); } to { transform: scale(1); } }

  /* Ajout du style pour le zoom sur l'image de fallback */
  :global(.fallback-zoomed) {
    object-fit: cover !important;
    object-position: center top !important;
    transform: scale(1.1);
    filter: saturate(0.9);
    width: 100% !important;
    height: 100% !important;
    display: block !important; /* Force l'affichage */
  }
</style>