// Vite: one page for both windows (index.html; the settings window is index.html#settings).
// Tauri serves the built files from dist/; in development `tauri dev` loads the dev server.
import { defineConfig } from 'vitest/config';
import react from '@vitejs/plugin-react';

export default defineConfig({
  plugins: [react()],
  clearScreen: false,
  server: { port: 1420, strictPort: true },
  build: { target: 'es2022', outDir: 'dist', emptyOutDir: true, chunkSizeWarningLimit: 2000 },
  test: { include: ['test/**/*.test.ts'] },
});
