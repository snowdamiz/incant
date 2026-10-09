import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';

// The shell must work offline: every asset (fonts, icons) is bundled locally.
export default defineConfig({
  plugins: [react()],
  base: './',
  // assetsInlineLimit 0: never inline fonts as data: URIs, which the CSP (font-src 'self') blocks.
  build: { target: 'es2022', sourcemap: true, assetsInlineLimit: 0 },
  server: { port: 5173, strictPort: true },
});
