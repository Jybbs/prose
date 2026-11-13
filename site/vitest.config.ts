import path from 'node:path'

import { playwright }                   from '@vitest/browser-playwright'
import { configDefaults, defineConfig } from 'vitest/config'

import { VUE_JSX } from './.vitepress/lib/shared/constants.ts'
import * as paths  from './.vitepress/lib/shared/paths.ts'

const BUNDLE = {
  globalSetup : ['./.vitepress/tests/serve.ts'],
  hookTimeout : 120_000,
  testTimeout : 120_000
}

const FIT = '.vitepress/tests/pages/fit.test.ts'

const ROOT = paths.repoRoot(import.meta.url)

const READ_FROM_DISK = [
  'Cargo.{lock,toml}', 'crate/{Cargo,pyproject}.toml', 'crate/src/**', 'crate/tests/fixtures/**',
  'site/**/*.{css,md}', 'site/.vitepress/tests/**/fixtures/**', 'site/{bun.lock,patches/**}',
  'site/public/**'
].map(glob => path.join(ROOT, glob))

export default defineConfig({
  cacheDir : paths.viteCacheDir(import.meta.url),
  test     : {
    environment         : 'node',
    forceRerunTriggers  : [...configDefaults.forceRerunTriggers, ...READ_FROM_DISK],
    globals             : true,
    pool                : 'threads',
    reporters           : process.env.GITHUB_ACTIONS
                        ? ['default', ['github-actions', { jobSummary: { enabled: false } }]]
                        : ['default'],
    resolveSnapshotPath : (testPath, extension) => testPath + extension,
    restoreMocks        : true,
    root                : import.meta.dirname,
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
          ...BUNDLE,
          exclude : [...configDefaults.exclude, FIT],
          include : ['.vitepress/tests/pages/**/*.test.ts'],
          name    : 'pages'
        }
      },

      {
        test: {
          ...BUNDLE,
          include : [FIT],
          name    : 'fit'
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
