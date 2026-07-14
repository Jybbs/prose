import vue                              from '@vitejs/plugin-vue'
import { configDefaults, defineConfig } from 'vitest/config'

export default defineConfig({
  test: {
    environment         : 'node',
    globals             : true,
    reporters           : process.env.GITHUB_ACTIONS
                        ? ['default', ['github-actions', { jobSummary: { enabled: false } }]]
                        : ['default'],
    resolveSnapshotPath : (testPath, extension) => testPath + extension,
    root                : import.meta.dirname,

    projects: [
      {
        extends : true,
        plugins : [vue()],
        test    : {
          exclude : [...configDefaults.exclude, '.vitepress/tests/wasm/**'],
          include : ['.vitepress/tests/**/*.test.ts'],
          name    : 'docs'
        }
      },

      {
        extends : true,
        test    : {
          include : ['.vitepress/tests/wasm/**/*.test.ts'],
          name    : 'wasm'
        }
      }
    ],

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
        '.vitepress/lib/sandbox/highlight.ts',
        '.vitepress/lib/sandbox/load-module.ts',
        '.vitepress/lib/sandbox/serve-plugin.ts',
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
