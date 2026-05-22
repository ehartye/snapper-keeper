// Self-hosted webfonts via fontsource — bundled into the Vite build so the
// app works offline and never pings Google. Imported for side-effects only;
// each module injects @font-face declarations and bundles the .woff2 files
// alongside the JS bundle.
//
// We only pull the latin subset of each face (the default import grabs every
// language subset, which roughly 5x's the font payload).

// Holographic Dreamcore: Rubik Bubbles (display, single weight) + Fredoka (body)
import '@fontsource/rubik-bubbles/latin-400.css';
import '@fontsource/fredoka/latin-400.css';
import '@fontsource/fredoka/latin-500.css';
import '@fontsource/fredoka/latin-600.css';
import '@fontsource/fredoka/latin-700.css';

// Memphis Machine: Bungee (display, single weight) + IBM Plex Mono (body)
import '@fontsource/bungee/latin-400.css';
// IBM Plex Mono — memphis body + robotic body
import '@fontsource/ibm-plex-mono/latin-400.css';
import '@fontsource/ibm-plex-mono/latin-500.css';
import '@fontsource/ibm-plex-mono/latin-600.css';
import '@fontsource/ibm-plex-mono/latin-700.css';

// Mr Robotic: VT323 (display, single weight). Body reuses IBM Plex Mono above.
import '@fontsource/vt323/latin-400.css';

// Corporate Overlord: Big Shoulders Display (display) + Archivo (body)
import '@fontsource/big-shoulders-display/latin-600.css';
import '@fontsource/big-shoulders-display/latin-800.css';
import '@fontsource/big-shoulders-display/latin-900.css';
import '@fontsource/archivo/latin-400.css';
import '@fontsource/archivo/latin-500.css';
import '@fontsource/archivo/latin-600.css';
import '@fontsource/archivo/latin-700.css';
import '@fontsource/archivo/latin-800.css';
