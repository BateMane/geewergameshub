/** @type {import('tailwindcss').Config} */
export default {
  content: ['./src/**/*.{html,js,svelte,ts}'],
  theme: {
    extend: {
      // On ajoute nos propres couleurs ici
      colors: {
        discord: {
          main: '#36393f',    // Le fond gris principal
          darker: '#2f3136',  // Pour les cartes/barres latérales
          light: '#40444b',   // Pour les survols
          text: '#dcddde',    // Le texte blanc cassé
          muted: '#b9bbbe',   // Le texte gris secondaire
          accent: '#5865F2',  // Le "Blurple" de Discord (ou utilise du vert si tu préfères)
        }
      },
      fontFamily: {
        // Une police moderne sans-serif
        sans: ['Inter', 'Roboto', 'Helvetica Neue', 'Arial', 'sans-serif'],
      }
    },
  },
  plugins: [],
}