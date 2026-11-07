import { playwright }                   from '@vitest/browser-playwright'
import { configDefaults, defineConfig } from 'vitest/config'

import { VUE_JSX }        from './.vitepress/lib/shared/constants'
import * as paths         from './.vitepress/lib/shared/paths'
import { SweepSequencer } from './.vitepress/tests/sequencer'

export default defineConfig({
  cacheDir : paths.viteCacheDir(import.meta.url),
  test     : {
    environment         : 'node',
    globals             : true,
    pool                : 'threads',
    reporters           : process.env.GITHUB_ACTIONS
                        ? ['default', ['github-actions', { jobSummary: { enabled: false } }]]
                        : ['default'],
    resolveSnapshotPath : (testPath, extension) => testPath + extension,
    restoreMocks        : true,
    root                : import.meta.dirname,
    sequence            : { sequencer: SweepSequencer },
    unstubEnvs          : true,
    unstubGlobals       : true,

    projects: [
      {
        oxc  : { jsx: VUE_JSX },
        test : {
          exclude    : [...configDefaults.exclude, '.vitepress/tests/pages/**', '.vitepress/tests/render/**', '.vitepress/tests/wasm/**'],
          include    : ['.vitepress/tests/**/*.test.ts'],
          name       : 'docs',
          setupFiles : ['./.vitepress/tests/setup.ts']
        }
      },

      {
        test: {
          globalSetup : ['./.vitepress/tests/serve.ts'],
          hookTimeout : 120_000,
          include     : ['.vitepress/tests/pages/**/*.test.ts'],
          name        : 'pages',
          testTimeout : 120_000
        }
      },

      {
        test: {
          globalSetup : ['./.vitepress/tests/serve.ts'],
          include     : ['.vitepress/tests/render/**/*.test.ts'],
          name        : 'render',
          testTimeout : 30_000
        }
      },

      {
        test: {
          include : ['.vitepress/tests/wasm/**/*.test.ts'],
          name    : 'wasm',
          browser : {
            enabled            : true,
            headless           : true,
            instances          : [{ browser: 'chromium' }],
            provider           : playwright(),
            screenshotFailures : false
          }
        }
      }
    ],

    coverage: {
      provider         : 'v8',
      reporter         : ['text', 'lcovonly'],
      reportsDirectory : paths.cacheDirFrom(paths.repoRoot(import.meta.url), 'coverage/site'),
      include          : ['.vitepress/kit/**', '.vitepress/lib/**', 'prototypes/**/*.{ts,tsx}'],

      exclude: [
        '.vitepress/kit/**/*.catalog.tsx',
        '.vitepress/lib/**/*.data.ts',
        '.vitepress/lib/og/render/build.ts',
        '.vitepress/lib/og/render/pool.ts',
        '.vitepress/lib/og/render/resvg-worker.mjs',
        '.vitepress/lib/sandbox/load-module.ts'
      ],

      thresholds: {
        branches   : 90,
        functions  : 95,
        lines      : 95,
        statements : 95,
        perFile    : { branches: 50, functions: 70, lines: 70, statements: 70 }
      }
    }
  }
})
