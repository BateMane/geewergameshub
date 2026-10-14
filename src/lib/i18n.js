import { register, init, getLocaleFromNavigator } from 'svelte-i18n';

register('fr', () => Promise.resolve({
  "header": {
    "welcome": "Bienvenue, vous avez",
    "games_ready": "jeux prêts.",
    "add_game": "Ajouter",
    "random_game": "Jeu Aléatoire",
    "settings": "Paramètres",
    "view_grid": "Vue Grille",
    "view_list": "Vue Liste",
    "sort_az": "A-Z",
    "sort_za": "Z-A",
    "sort_recent": "Récents",
    "search": "Rechercher un jeu...",
    "refresh": "Rescanner la bibliothèque (F5)"
  },
  "tabs": {
    "all": "Tout",
    "fav": "Favoris",
    "custom": "Perso",
    "hidden": "Masqués"
  },
  "recent": {
    "title": "Reprendre",
    "library": "Bibliothèque"
  },
  "empty": {
    "no_results": "Aucun jeu ne correspond à ta recherche.",
    "no_games": "Aucun jeu ici pour l'instant.",
    "clear_search": "Effacer la recherche"
  },
  "toast": {
    "launching": "Lancement de",
    "added": "a été ajouté !",
    "hidden": "Jeu masqué (onglet Masqués pour le retrouver)",
    "unhidden": "Jeu de nouveau visible"
  },
  "game_card": {
    "play": "JOUER",
    "folder": "Dossier",
    "open": "Ouvrir",
    "details": "Détails",
    "favorite": "Favori"
  },
  "modal_add": {
    "title": "AJOUTER UN JEU",
    "label_title": "Titre du jeu",
    "placeholder_title": "Ex: Minecraft",
    "label_exe": "Exécutable (.exe)",
    "placeholder_path": "Chemin du fichier...",
    "label_img": "Image (Optionnel)",
    "placeholder_img": "Chemin de l'image...",
    "cancel": "Annuler",
    "confirm": "Ajouter"
  },
  "settings": {
    "title": "Paramètres",
    "tabs": {
      "general": "Général",
      "appearance": "Apparence",
      "about": "À Propos"
    },
    "close": "Fermer",
    "save": "Sauvegarder",
    "general": {
      "autostart_title": "Lancement automatique",
      "autostart_desc": "Démarrer Geewer GameHub avec le système",
      "drives_title": "Disques à scanner",
      "drives_desc": "Sélectionnez les lecteurs où vos jeux sont installés pour optimiser le scan.",
      "no_drives": "Aucun disque détecté automatiquement.",
      "drive": "Disque"
    },
    "appearance": {
      "presets_title": "Thèmes Prédéfinis",
      "custom_title": "Personnalisation Avancée",
      "accent": "Couleur d'Accentuation",
      "card_bg": "Couleur des Cartes",
      "bg_top": "Fond (Haut)",
      "bg_bottom": "Fond (Bas)",
      "text_main": "Texte Principal"
    },
    "about": {
      "version": "Version",
      "report_btn": "Signaler un Bug / Suggestion",
      "coffee_btn": "M'offrir un café"
    }
  },
  "details": {
    "loading": "Chargement...",
    "developer": "Développeur",
    "date": "Date",
    "unknown": "Inconnu",
    "metacritic": "Metacritic",
    "mods_title": "MODS DISPONIBLES",
    "hltb_title": "Temps de jeu",
    "hltb_story": "Histoire",
    "hltb_extra": "Extra",
    "hltb_100": "100%",
    "source_label": "Source",
    "play_btn": "LANCER",
    "folder_btn": "DOSSIER",
    "open_btn": "OUVRIR",
    "no_info": "Aucune information supplémentaire disponible pour ce jeu.",
    "last_played": "Joué",
    "screenshots": "Captures d'écran",
    "hide": "Masquer de la bibliothèque",
    "unhide": "Afficher dans la bibliothèque",
    "remove": "Retirer ce jeu",
    "remove_confirm": "Retirer ce jeu de la bibliothèque ? (Le jeu lui-même n'est pas désinstallé)"
  },
  "autostart_popup": {
    "title": "Lancer au démarrage ?",
    "msg": "Voulez-vous que GameHub se lance automatiquement avec Windows ?",
    "dont_ask": "Ne plus demander",
    "no": "Non",
    "yes": "Oui"
  },
  "rate_popup": {
    "text": "L'application vous plaît? Notez la sur le Microsoft Store!",
    "btn": "Noter maintenant"
  },
  "error": "Erreur"
}));

register('en', () => Promise.resolve({
  "header": {
    "welcome": "Welcome, you have",
    "games_ready": "games ready.",
    "add_game": "Add Game",
    "random_game": "Random Game",
    "settings": "Settings",
    "view_grid": "Grid View",
    "view_list": "List View",
    "sort_az": "A-Z",
    "sort_za": "Z-A",
    "sort_recent": "Recent",
    "search": "Search a game...",
    "refresh": "Rescan library (F5)"
  },
  "tabs": {
    "all": "All",
    "fav": "Favorites",
    "custom": "Custom",
    "hidden": "Hidden"
  },
  "recent": {
    "title": "Jump back in",
    "library": "Library"
  },
  "empty": {
    "no_results": "No game matches your search.",
    "no_games": "No games here yet.",
    "clear_search": "Clear search"
  },
  "toast": {
    "launching": "Launching",
    "added": "has been added!",
    "hidden": "Game hidden (see the Hidden tab)",
    "unhidden": "Game visible again"
  },
  "game_card": {
    "play": "PLAY",
    "folder": "Folder",
    "open": "Open",
    "details": "Details",
    "favorite": "Favorite"
  },
  "modal_add": {
    "title": "ADD A GAME",
    "label_title": "Game Title",
    "placeholder_title": "Ex: Minecraft",
    "label_exe": "Executable (.exe)",
    "placeholder_path": "File path...",
    "label_img": "Image (Optional)",
    "placeholder_img": "Image path...",
    "cancel": "Cancel",
    "confirm": "Add"
  },
  "settings": {
    "title": "Settings",
    "tabs": {
      "general": "General",
      "appearance": "Appearance",
      "about": "About"
    },
    "close": "Close",
    "save": "Save",
    "general": {
      "autostart_title": "Auto-start",
      "autostart_desc": "Start Geewer GameHub with Windows",
      "drives_title": "Drives to scan",
      "drives_desc": "Select drives where your games are installed to optimize scanning.",
      "no_drives": "No drives detected automatically.",
      "drive": "Drive"
    },
    "appearance": {
      "presets_title": "Preset Themes",
      "custom_title": "Advanced Customization",
      "accent": "Accent Color",
      "card_bg": "Card Color",
      "bg_top": "Background (Top)",
      "bg_bottom": "Background (Bottom)",
      "text_main": "Primary Text"
    },
    "about": {
      "version": "Version",
      "report_btn": "Report a Bug / Suggestion",
      "coffee_btn": "Buy me a coffee"
    }
  },
  "details": {
    "loading": "Loading...",
    "developer": "Developer",
    "date": "Date",
    "unknown": "Unknown",
    "metacritic": "Metacritic",
    "mods_title": "AVAILABLE MODS",
    "hltb_title": "Time to Beat",
    "hltb_story": "Main",
    "hltb_extra": "Extra",
    "hltb_100": "100%",
    "source_label": "Source",
    "play_btn": "PLAY",
    "folder_btn": "FOLDER",
    "open_btn": "OPEN",
    "no_info": "No additional information available for this game.",
    "last_played": "Played",
    "screenshots": "Screenshots",
    "hide": "Hide from library",
    "unhide": "Show in library",
    "remove": "Remove this game",
    "remove_confirm": "Remove this game from the library? (The game itself is not uninstalled)"
  },
  "autostart_popup": {
    "title": "Launch on startup?",
    "msg": "Do you want GameHub to start automatically with Windows?",
    "dont_ask": "Don't ask again",
    "no": "No",
    "yes": "Yes"
  },
  "rate_popup": {
    "text": "Enjoying the app? Rate it on the Microsoft Store!",
    "btn": "Rate now"
  },
  "error": "Error"
}));

init({
  fallbackLocale: 'en',
  initialLocale: getLocaleFromNavigator(),
});