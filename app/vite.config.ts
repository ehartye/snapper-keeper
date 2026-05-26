import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { execSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { dirname, resolve } from 'node:path';

const __dirname = dirname(fileURLToPath(import.meta.url));

function gitShortSha(): string {
  try {
    return execSync('git rev-parse --short HEAD', { encoding: 'utf8' }).trim();
  } catch {
    return 'dev';
  }
}

function updaterFingerprint(): string {
  try {
    const cfgRaw = readFileSync(
      resolve(__dirname, 'src-tauri/tauri.conf.json'),
      'utf8',
    );
    const cfg = JSON.parse(cfgRaw) as {
      plugins?: { updater?: { pubkey?: string } };
    };
    const pubkeyB64 = cfg.plugins?.updater?.pubkey ?? '';
    const decoded = Buffer.from(pubkeyB64, 'base64').toString('utf8');
    const match = /minisign public key:\s*([0-9A-Fa-f]+)/i.exec(decoded);
    return match?.[1] ?? 'unknown';
  } catch {
    return 'unknown';
  }
}

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: {
    port: 5173,
    strictPort: true,
  },
  envPrefix: ['VITE_', 'TAURI_'],
  define: {
    __GIT_SHA__: JSON.stringify(gitShortSha()),
    __UPDATER_FINGERPRINT__: JSON.stringify(updaterFingerprint()),
  },
  build: {
    target: 'es2022',
    sourcemap: true,
  },
});
