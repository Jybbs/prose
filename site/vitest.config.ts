import vue              from '@vitejs/plugin-vue'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  plugins: [vue()],
  test: {
    environment : 'node',
    globals     : true,
    root        : import.meta.dirname,
    include     : ['.vitepress/tests/**/*.test.ts'],
    exclude     : ['.vitepress/tests/wasm/**'],
    reporters   : process.env.GITHUB_ACTIONS ? ['default', 'github-actions'] : ['default'],
    resolveSnapshotPath : (testPath, extension) => testPath + extension,
    coverage: {
      provider         : 'v8',
      reporter         : ['text', 'lcovonly'],
      reportsDirectory : 'coverage',
      include          : ['.vitepress/lib/**'],
      exclude: [
        '.vitepress/lib/**/*.data.ts',
        '.vitepress/lib/og/render/build.ts',
        '.vitepress/lib/og/render/cache.ts',
        '.vitepress/lib/og/render/pool.ts',
        '.vitepress/lib/og/render/resvg-worker.mjs',
        '.vitepress/lib/rules/config-presets.ts',
        '.vitepress/lib/shared/fixture-tab.ts',
        '.vitepress/lib/shared/tools.ts'
      ],
      thresholds: {
        branches   : 90,
        functions  : 95,
        lines      : 95,
        statements : 95
      }
    }
  }
})
