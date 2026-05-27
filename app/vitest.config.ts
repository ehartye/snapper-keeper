import { defineConfig } from 'vitest/config';

export default defineConfig({
  define: {
    __GIT_SHA__: JSON.stringify('test'),
    __UPDATER_FINGERPRINT__: JSON.stringify('testfingerprint'),
    __STORE_EDITION__: JSON.stringify(false),
  },
  test: {
    environment: 'happy-dom',
    globals: true,
    setupFiles: ['./src/test/setup.ts'],
    include: ['src/**/*.test.{ts,tsx}'],
    coverage: {
      provider: 'v8',
      reporter: ['text', 'json-summary'],
      include: ['src/**/*.{ts,tsx}'],
      exclude: [
        'src/**/*.test.{ts,tsx}',
        'src/test/**',
        'src/main.tsx',
        'src/lib/fonts.ts',
        // Window-shell entry points / pure visual containers — tested
        // indirectly by their child components.
        'src/App.tsx',
        // Konva-bound code can't be meaningfully unit-tested in happy-dom
        // (the canvas API isn't implemented). Covered by manual smoke tests.
        'src/windows/annotate/shapes/**',
        'src/windows/annotate/AnnotateCanvas.tsx',
        'src/windows/annotate/AnnotateWindow.tsx',
        'src/windows/annotate/useDrawing.tsx',
        // Heavy window-platform wiring; the testable bits are covered via
        // their inner components or the relevant Rust commands.
        'src/windows/capture-overlay/**',
      ],
    },
  },
  resolve: {
    alias: {
      // Use the source files directly (matches the existing workspace setup
      // where each @snk/* package exposes ./src/index.ts as main).
      '@snk/library': new URL('../packages/snk-library/src/index.ts', import.meta.url).pathname,
      '@snk/capture': new URL('../packages/snk-capture/src/index.ts', import.meta.url).pathname,
      '@snk/clipboard': new URL('../packages/snk-clipboard/src/index.ts', import.meta.url).pathname,
      '@snk/annotate': new URL('../packages/snk-annotate/src/index.ts', import.meta.url).pathname,
      '@snk/updater': new URL('../packages/snk-updater/src/index.ts', import.meta.url).pathname,
    },
  },
});
