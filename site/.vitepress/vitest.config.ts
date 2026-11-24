import path from 'node:path'

import { playwright }                   from '@vitest/browser-playwright'
import { groupIconVitePlugin }          from 'vitepress-plugin-group-icons'
import { configDefaults, defineConfig } from 'vitest/config'

import { readFromDiskPlugin } from './lib/config/read-from-disk.ts'
import { VUE_JSX }            from './lib/shared/constants.ts'
import * as paths             from './lib/shared/paths.ts'

const BUNDLE = {
  globalSetup : ['./.vitepress/tests/serve.ts'],
  hookTimeout : 120_000,
  testTimeout : 120_000
}
export const CRATE   = 'crate/src/**/*.rs'
const FIT            = '.vitepress/tests/pages/fit.test.ts'
const GRAPH          = 'site/.vitepress/lib/**/*.{mjs,ts}'
const PAGES          = 'site/pages/**/*.md'
const ROOT           = paths.repoRoot(import.meta.url)
const RULES          = 'crate/src/rules/**/*.rs'
const SCRIPTS        = 'site/.vitepress/{kit,lib,theme}/**/*.{ts,tsx}'
const STYLES         = 'site/.vitepress/{kit,theme}/**/*.css'
export const READERS = {
  'site/.vitepress/config.ts'                               : [GRAPH],
  'site/.vitepress/lib/exit-codes/sources.ts'               : [CRATE],
  'site/.vitepress/lib/fixtures/variants.ts'                : [CRATE],
  'site/.vitepress/lib/identity/identity.data.ts'           : [SCRIPTS, STYLES, 'site/.vitepress/config.ts'],
  'site/.vitepress/lib/kit/catalog.data.ts'                 : ['site/.vitepress/kit/**/*.{ts,tsx}'],
  'site/.vitepress/lib/landing/landing-typing-demo.data.ts' : [CRATE],
  'site/.vitepress/lib/reference/cli-help.ts'               : [CRATE],
  'site/.vitepress/lib/reference/output-samples.data.ts'    : [CRATE],
  'site/.vitepress/lib/reference/section-glance.data.ts'    : [PAGES],
  'site/.vitepress/lib/rules/discovery.ts'                  : [RULES],
  'site/.vitepress/lib/rules/pipeline.ts'                   : [CRATE],
  'site/.vitepress/lib/shared/content-page.ts'              : [PAGES],
  'site/.vitepress/lib/shared/rule-schema.ts'               : [CRATE],
  'site/.vitepress/lib/tokens/destinations.ts'              : [PAGES],
  'site/.vitepress/tests/config/edit-link.test.ts'          : [PAGES],
  'site/.vitepress/tests/config/head.test.ts'               : [PAGES],
  'site/.vitepress/tests/config/readers.test.ts'            : ['site/.vitepress/**/*.{mjs,ts,tsx}'],
  'site/.vitepress/tests/og/render.test.ts'                 : ['site/.vitepress/tests/og/cards/*/page.ts'],
  'site/.vitepress/tests/reference/section-glance.test.ts'  : [PAGES],
  'site/.vitepress/tests/shared/palette.test.ts'            : [STYLES],
  'site/.vitepress/tests/shared/parity.test.ts'             : [PAGES, STYLES],
  'site/.vitepress/tests/shared/render-graph.test.ts'       : [GRAPH],
  'site/.vitepress/tests/shared/stylesheets.test.ts'        : [SCRIPTS],
  'site/.vitepress/tests/shared/tool-chips.test.ts'         : [PAGES],
  'site/.vitepress/tests/support.ts'                        : [STYLES],
  'site/.vitepress/tests/suppression/scopes.test.ts'        : [PAGES],
  'site/.vitepress/tests/vocabulary.ts'                     : [STYLES],

  'site/.vitepress/tests/identity/declarations.test.ts': [
    'site/.vitepress/lib/identity/*.ts', 'site/.vitepress/theme/components/identity/**/*.{css,tsx}'
  ],

  'site/.vitepress/tests/integrations/hue-limit.test.ts': [
    'site/.vitepress/theme/components/integrations/**/*.{css,tsx}'
  ],

  'site/.vitepress/tests/shared/classes.test.ts': [
    'site/.vitepress/{kit,lib,tests,theme}/**/*.{ts,tsx}'
  ],

  'site/.vitepress/tests/shared/emblems.test.ts': [
    'site/.vitepress/theme/components/rules/PipelineOrder.tsx'
  ],

  'site/.vitepress/tests/tailwind.ts': [
    'site/.vitepress/kit/**/*.{ts,tsx}', 'site/.vitepress/lib/**/*.ts', 'site/.vitepress/theme/**/*.tsx',
    'site/pages/index.md', STYLES
  ]
}
const RERUN_TRIGGERS = [
  '.github/scripts/platforms.toml', 'Cargo.{lock,toml}', 'crate/{Cargo,pyproject}.toml',
  'crate/tests/fixtures/**', 'site/.vitepress/{assets,patches}/**', 'site/.vitepress/tests/**/fixtures/*/**',
  'site/bun.lock', 'site/public/**'
].map(glob => path.join(ROOT, glob))

export default defineConfig({
  cacheDir : paths.viteCacheDir(import.meta.url),
  test     : {
    environment         : 'node',
    expect              : { requireAssertions: true },
    forceRerunTriggers  : [...configDefaults.forceRerunTriggers, ...RERUN_TRIGGERS],
    globals             : true,
    pool                : 'threads',
    reporters           : process.env.GITHUB_ACTIONS
                        ? ['default', ['github-actions', { jobSummary: { enabled: false } }]]
                        : ['default'],
    resolveSnapshotPath : (testPath, extension) => testPath + extension,
    restoreMocks        : true,
    root                : path.dirname(import.meta.dirname),
    unstubEnvs          : true,
    unstubGlobals       : true,

    projects: [
      {
        oxc     : { jsx: VUE_JSX },
        plugins : [groupIconVitePlugin(), readFromDiskPlugin(READERS, ROOT)],
        test    : {
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
            provider           : playwright({ launchOptions: { args: ['--mute-audio'] } }),
            screenshotFailures : false
          }
        }
      }
    ],

    coverage: {
      provider         : 'v8',
      reporter         : ['text', 'lcovonly'],
      reportsDirectory : paths.cacheDirFrom(ROOT, 'coverage/site'),
      include          : ['.vitepress/kit/**', '.vitepress/lib/**'],

      exclude: [
        '.vitepress/kit/**/*.catalog.tsx',
        '.vitepress/lib/**/*.data.ts',
        '.vitepress/lib/og/render/build.ts',
        '.vitepress/lib/og/render/pool.ts',
        '.vitepress/lib/og/render/resvg-worker.mjs',
        '.vitepress/lib/readme/record.ts',
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
