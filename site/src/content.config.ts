import { defineCollection } from 'astro:content'
import { file, glob }       from 'astro/loaders'
import { docsSchema }       from '@astrojs/starlight/schema'

import { docsLoaderWithIntegrity } from './lib/content/docs-loader'
import { fixturesLoader }          from './lib/content/fixtures'
import * as schema                 from './lib/content/schemas'

const data = (name: string): string => `src/data/${name}.yaml`

const compositionLoader = glob({
  base       : '../crate/tests/fixtures/composition',
  generateId : ({ entry }) => entry.replace(/\/config\.toml$/, ''),
  pattern    : '*/config.toml'
})

export const collections = {
  composition       : defineCollection({ loader: compositionLoader,                  schema: schema.composition }),
  directives        : defineCollection({ loader: file(data('directives')),           schema: schema.directive }),
  docs              : defineCollection({ loader: docsLoaderWithIntegrity(),          schema: docsSchema({ extend: schema.docsExtension }) }),
  editorConfigs     : defineCollection({ loader: file(data('editor-configs')),       schema: schema.editorConfig }),
  exitCodes         : defineCollection({ loader: file(data('exit-codes')),           schema: schema.exitCode }),
  fixtures          : defineCollection({ loader: fixturesLoader(),                   schema: schema.fixture }),
  glossary          : defineCollection({ loader: file(data('glossary')),             schema: schema.glossary }),
  landingSurfaces   : defineCollection({ loader: file(data('landing-surfaces')),     schema: schema.landingSurface }),
  landingWorkflow   : defineCollection({ loader: file(data('landing-workflow')),     schema: schema.landingStep }),
  ruleConfigPresets : defineCollection({ loader: file(data('rule-config-presets')),  schema: schema.ruleConfigPreset }),
  shellCompletions  : defineCollection({ loader: file(data('shell-completions')),    schema: schema.shellCompletion }),
  tokenIndex        : defineCollection({ loader: file(data('token-index')),          schema: schema.tokenIndex }),
  tools             : defineCollection({ loader: file(data('tools')),                schema: schema.tool })
}
