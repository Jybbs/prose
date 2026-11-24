import path from 'node:path'

import { playwright }                   from '@vitest/browser-playwright'
import { groupIconVitePlugin }          from 'vitepress-plugin-group-icons'
import { configDefaults, defineConfig } from 'vitest/config'

import { readFromDiskPlugin } from './lib/config/read-from-disk.ts'
import { VUE_JSX }            from './lib/shared/constants.ts'
import * as paths             from './lib/shared/paths.ts'

const TESTS  = paths.siteTestsDir(import.meta.url)
const BUNDLE = {
  globalSetup : [path.join(TESTS, 'serve.ts')],
  hookTimeout : 120_000,
  testTimeout : 120_000
}
export const CRATE   = 'crate/src/**/*.rs'
const FIT            = 'pages/fit.test.ts'
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
  'tests/site/config/edit-link.test.ts'                     : [PAGES],
  'tests/site/config/head.test.ts'                          : [PAGES],
  'tests/site/config/readers.test.ts'                       : ['{site/.vitepress,tests/site}/**/*.{mjs,ts,tsx}'],
  'tests/site/og/render.test.ts'                            : ['tests/site/og/cards/*/page.ts'],
  'tests/site/reference/section-glance.test.ts'             : [PAGES],
  'tests/site/shared/classes.test.ts'                       : [SCRIPTS, 'tests/site/**/*.{ts,tsx}'],
  'tests/site/shared/palette.test.ts'                       : [STYLES],
  'tests/site/shared/parity.test.ts'                        : [PAGES, STYLES],
  'tests/site/shared/render-graph.test.ts'                  : [GRAPH],
  'tests/site/shared/stylesheets.test.ts'                   : [SCRIPTS],
  'tests/site/shared/tool-chips.test.ts'                    : [PAGES],
  'tests/site/support.ts'                                   : [STYLES],
  'tests/site/suppression/scopes.test.ts'                   : [PAGES],
  'tests/site/vocabulary.ts'                                : [STYLES],

  'tests/site/identity/declarations.test.ts': [
    'site/.vitepress/lib/identity/*.ts', 'site/.vitepress/theme/components/identity/**/*.{css,tsx}'
  ],

  'tests/site/integrations/hue-limit.test.ts': [
    'site/.vitepress/theme/components/integrations/**/*.{css,tsx}'
  ],

  'tests/site/shared/emblems.test.ts': [
    'site/.vitepress/theme/components/rules/PipelineOrder.tsx'
  ],

  'tests/site/tailwind.ts': [
    'site/.vitepress/kit/**/*.{ts,tsx}', 'site/.vitepress/lib/**/*.ts', 'site/.vitepress/theme/**/*.tsx',
    'site/pages/index.md', STYLES
  ]
}
export const RERUN_TRIGGERS = [
  '.github/scripts/platforms.toml', 'Cargo.{lock,toml}', 'bun.lock', 'crate/{Cargo,pyproject}.toml',
  'site/.vitepress/{assets,patches}/**', 'site/public/**', 'tests/rust/fixtures/**', 'tests/site/**/fixtures/*/**'
].map(glob => path.join(ROOT, glob))

export default defineConfig({
  cacheDir : paths.viteCacheDir(import.meta.url),
  test     : {
    dir                 : TESTS,
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
          exclude    : [...configDefaults.exclude, 'pages/**', 'render/**', 'wasm/**'],
          include    : ['**/*.test.ts'],
          name       : 'docs',
          setupFiles : [path.join(TESTS, 'setup.ts')]
        }
      },

      {
        test: {
          ...BUNDLE,
          exclude : [...configDefaults.exclude, FIT],
          include : ['pages/**/*.test.ts'],
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
          globalSetup : BUNDLE.globalSetup,
          include     : ['render/**/*.test.ts'],
          name        : 'render',
          testTimeout : 30_000
        }
      },

      {
        test: {
          include : ['wasm/**/*.test.ts'],
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
