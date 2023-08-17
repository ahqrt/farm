import type { UserConfig } from '@farmfe/core';

function defineConfig(config: UserConfig) {
  return config;
}

export default defineConfig({
  compilation: {
    input: {
      index: './index.html',
    },
    resolve: {
      symlinks: true,
    },
    define: {
      BTN: 'Click me',
    },
    output: {
      path: './build',
    },
    sourcemap: false,
    persistentCache: true,
  },
  server: {
    hmr: true,
  },
  plugins: ['@farmfe/plugin-react', '@farmfe/plugin-sass'],
});
