# Geewer Game's Hub 1.0.1

![Version](https://img.shields.io/badge/version-1.0.0-blue?style=flat-square)
![Tauri](https://img.shields.io/badge/Tauri-2.0-orange?style=flat-square&logo=tauri)
![Svelte](https://img.shields.io/badge/Svelte-red?style=flat-square&logo=svelte)

Salut ! **Geewer Game's Hub**, c'est tout simplement un launcher pour rassembler tous tes jeux au même endroit.

J'en avais marre d'ouvrir 4 ou 5 applications différentes (Steam, Epic, Ubisoft...) juste pour retrouver un jeu. Du coup, j'ai créé cette app : elle scanne ton PC, trouve tes jeux installés peu importe la boutique, et te les affiche dans une belle interface unifiée.

C'est léger, ça ne tourne pas en fond pour rien, et c'est codé avec **Tauri (Rust)** pour être le plus rapide possible.

<img width="1917" height="985" alt="interface" src="https://github.com/user-attachments/assets/0278dc51-3524-4724-95e5-86075219ed86" />

---

## ⚡ Qu'est-ce qu'on peut faire avec ?

L'idée est de rendre la gestion de ta bibliothèque plus agréable :

* **Tout tes jeux au même endroit** : L'app détecte automatiquement les jeux Steam, Epic Games, GOG, Ubisoft Connect et EA App.
* **Ajout manuel** : Tu peux aussi ajouter toi-même des jeux qui ne viennent pas de ces boutiques (jeux indés, émulateurs, etc.).
* **Infos utiles** : Quand tu cliques sur un jeu, tu vois les infos principales de celui-ci et si des mods sont dispos sur *Nexus Mods* ou *Thunderstore*.
* **C'est ton interface** : Tu peux changer les couleurs, le fond d'écran et le thème global pour que ça ressemble à ce que tu aimes.
* **Choix des disques** : Si tu as plusieurs disques durs, tu peux dire à l'app lesquels scanner pour éviter qu'elle cherche partout inutilement.

---

## 🛠️ Comment l'installer ou le tester ?

Si tu veux tester le projet ou modifier le code, c'est assez simple. Il te faut juste **Node.js** et **Rust** installés sur ta machine.

1.  **Récupère le projet**
    ```bash
    git clone [https://github.com/ton-pseudo/geewergameshub.git](https://github.com/ton-pseudo/geewergameshub.git)
    cd geewergameshub
    ```

2.  **Installe ce qu'il faut**
    ```bash
    npm install
    ```

3.  **Lance l'app**
    ```bash
    npm run tauri dev
    ```

Et voilà, la fenêtre devrait s'ouvrir !

---

## 💻 C'est fait comment ?

Pour les curieux, voici la stack technique :
* **Frontend** : SvelteKit + Tailwind CSS (pour que ce soit fluide et joli).
* **Backend** : Rust via Tauri (pour gérer le système de fichiers, le scan des jeux et les performances).

Si tu as des idées pour améliorer le hub ou si tu trouves un bug, n'hésite pas à ouvrir une *Issue* ou à utiliser le bouton de feedback directement dans les paramètres de l'app !
