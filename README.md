# Geewer Game's Hub 1.0.4

![Version](https://img.shields.io/badge/version-1.0.4-blue?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-2.0-orange?style=flat-square&logo=tauri)
![Svelte](https://img.shields.io/badge/Svelte-red?style=flat-square&logo=svelte)

<div align="center">

**[🇫🇷 Français](#-français)** | **[🇬🇧 English](#-english)**

</div>

<img width="1917" height="985" alt="interface" src="https://github.com/user-attachments/assets/0278dc51-3524-4724-95e5-86075219ed86" />

---

## 🇫🇷 Français

Salut ! **Geewer Game's Hub**, c'est tout simplement un launcher pour rassembler tous tes jeux au même endroit.

J'en avais marre d'ouvrir 4 ou 5 applications différentes (Steam, Epic, Ubisoft...) juste pour retrouver un jeu. Du coup, j'ai créé cette app : elle scanne ton PC, trouve tes jeux installés peu importe la boutique, et te les affiche dans une belle interface unifiée.

C'est léger, ça ne tourne pas en fond pour rien, et c'est codé avec **Tauri (Rust)** pour être le plus rapide possible.

### ⚡ Qu'est-ce qu'on peut faire avec ?

L'idée est de rendre la gestion de ta bibliothèque plus agréable :

* **Tout tes jeux au même endroit** : L'app détecte automatiquement les jeux Steam, Epic Games, GOG, Ubisoft Connect et EA App.
* **Ajout manuel** : Tu peux aussi ajouter toi-même des jeux qui ne viennent pas de ces boutiques (jeux indés, émulateurs, etc.).
* **Infos utiles** : Quand tu cliques sur un jeu, tu vois les infos principales de celui-ci et si des mods sont dispos sur *Nexus Mods* ou *Thunderstore*.
* **C'est ton interface** : Tu peux changer les couleurs, le fond d'écran et le thème global pour que ça ressemble à ce que tu aimes.
* **Choix des disques** : Si tu as plusieurs disques durs, tu peux dire à l'app lesquels scanner pour éviter qu'elle cherche partout inutilement.

### 🛠️ Comment l'installer ou le tester ?

🚀 **Téléchargement facile** : L'application est disponible directement sur le **Microsoft Store** !  
👉 [**Clique ici pour télécharger Geewer Game's Hub**](https://apps.microsoft.com/detail/9N9CF4NLLQZ7?hl=fr&gl=FR&ocid=pdpshare)

Si tu préfères tester le projet depuis le code source ou le compiler toi-même, il te faut **Node.js** et **Rust** installés sur ta machine.

1.  **Récupère le projet**
    ```bash
    git clone [https://github.com/ton-pseudo/geewergameshub.git](https://github.com/ton-pseudo/geewergameshub.git)
    cd geewergameshub
    ```

2.  **Installe ce qu'il faut**
    ```bash
    npm install
    ```

3.  **Lance l'app en mode dev**
    ```bash
    npm run tauri dev
    ```

📦 **Où est l'installateur ?** Si tu lances une compilation complète (via `npm run tauri build`), tu trouveras l'exécutable d'installation généré dans ce dossier :
`\src-tauri\target\release\bundle`

### 💻 C'est fait comment ?

Pour les curieux, voici la stack technique :
* **Frontend** : SvelteKit + Tailwind CSS (pour que ce soit fluide et joli).
* **Backend** : Rust via Tauri (pour gérer le système de fichiers, le scan des jeux et les performances).

Si tu as des idées pour améliorer le hub ou si tu trouves un bug, n'hésite pas à ouvrir une *Issue* ou à utiliser le bouton de feedback directement dans les paramètres de l'app !

---

## 🇬🇧 English

Hi there! **Geewer Game's Hub** is simply a launcher designed to gather all your games in one place.

I was tired of opening 4 or 5 different applications (Steam, Epic, Ubisoft...) just to find a game. So, I created this app: it scans your PC, finds your installed games regardless of the store, and displays them in a beautiful unified interface.

It's lightweight, doesn't run in the background for no reason, and is coded with **Tauri (Rust)** to be as fast as possible.

### ⚡ What can you do with it?

The idea is to make managing your library more enjoyable:

* **All your games in one place**: The app automatically detects Steam, Epic Games, GOG, Ubisoft Connect, and EA App games.
* **Manual addition**: You can also manually add games that don't come from these stores (indie games, emulators, etc.).
* **Useful info**: When you click on a game, you see its main information and if mods are available on *Nexus Mods* or *Thunderstore*.
* **It's your interface**: You can change the colors, the background, and the global theme to make it look the way you like.
* **Drive selection**: If you have multiple hard drives, you can tell the app which ones to scan to avoid searching everywhere unnecessarily.

### 🛠️ How to install or test it?

🚀 **Easy Download**: The app is available directly on the **Microsoft Store**!  
👉 [**Click here to download Geewer Game's Hub**](https://apps.microsoft.com/detail/9N9CF4NLLQZ7?hl=fr&gl=FR&ocid=pdpshare)

If you want to test the project from source or compile it yourself, you need **Node.js** and **Rust** installed on your machine.

1.  **Clone the project**
    ```bash
    git clone [https://github.com/ton-pseudo/geewergameshub.git](https://github.com/your-username/geewergameshub.git)
    cd geewergameshub
    ```

2.  **Install dependencies**
    ```bash
    npm install
    ```

3.  **Launch the app in dev mode**
    ```bash
    npm run tauri dev
    ```

📦 **Where is the installer?** If you run a full build (via `npm run tauri build`), you will find the generated installation file in this folder:
`\src-tauri\target\release\bundle`

### 💻 How is it made?

For the curious, here is the tech stack:
* **Frontend**: SvelteKit + Tailwind CSS (to make it smooth and pretty).
* **Backend**: Rust via Tauri (to handle the file system, game scanning, and performance).

If you have ideas to improve the hub or if you find a bug, don't hesitate to open an *Issue* or use the feedback button directly in the app settings!