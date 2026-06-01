import { defineConfig } from 'vitest/config';
import { fileURLToPath } from 'node:url';

const resolvePath = (p: string) => fileURLToPath(new URL(p, import.meta.url));

// Standalone Vitest config — deliberately does NOT load the SvelteKit Vite
// plugin. The unit suites under test exercise plain `.ts` modules (stores, the
// WS client/bridge, utils); pulling in the full SvelteKit plugin would force
// us to boot the kit virtual-module graph for no benefit. SvelteKit's `$app`
// and `$env` virtual modules are stubbed via aliases below.
export default defineConfig({
  resolve: {
    alias: {
      $lib: resolvePath('./src/lib'),
      $stores: resolvePath('./src/lib/stores'),
      $components: resolvePath('./src/lib/components'),
      $api: resolvePath('./src/lib/api'),
      $ws: resolvePath('./src/lib/ws'),
      '$app/environment': resolvePath('./src/test/mocks/app-environment.ts'),
      '$env/dynamic/public': resolvePath('./src/test/mocks/env-dynamic-public.ts'),
    },
  },
  test: {
    environment: 'jsdom',
    globals: true,
    clearMocks: true,
    include: ['src/**/*.{test,spec}.ts'],
    setupFiles: ['./src/test/setup.ts'],
    coverage: {
      provider: 'v8',
      reportsDirectory: './coverage',
      include: ['src/lib/stores/**', 'src/lib/ws/**', 'src/lib/utils/**'],
      exclude: ['**/*.test.ts', '**/*.spec.ts'],
    },
  },
});
