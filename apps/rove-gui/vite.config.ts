import { defineConfig } from 'vite';
import vue from '@vitejs/plugin-vue';
import { realpathSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
// A local dependency cache may live on another disk. Permit that exact
// directory for CSS fonts/assets, never its parent or the entire data volume.
const project = fileURLToPath(new URL('.', import.meta.url));
const dependencies = realpathSync(fileURLToPath(new URL('./node_modules', import.meta.url)));
export default defineConfig({ plugins: [vue()], clearScreen: false, server: { fs: { allow: [project, dependencies] } } });
