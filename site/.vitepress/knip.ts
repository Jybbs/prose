import { parse }        from '@vue/compiler-sfc'
import { defineConfig } from 'knip/config'

const pageScript = (text: string): string => {
  const { descriptor } = parse(text)
  return [descriptor.script?.content, descriptor.scriptSetup?.content].filter(Boolean).join('\n')
}

export default defineConfig({
  entry              : [
    'site/**/*.md', 'site/.vitepress/**/*.data.ts', 'site/.vitepress/kit/**/*.catalog.tsx',
    'site/.vitepress/lib/readme/{cards,record}.ts'
  ],
  project            : ['site/.vitepress/**/*.{ts,tsx,mjs}', 'site/**/*.md', 'tests/site/**/*.{ts,tsx,mjs}'],
  compilers          : { md: pageScript },
  include            : ['binaries', 'cycles', 'dependencies', 'duplicates', 'enumMembers', 'exports', 'files', 'namespaceMembers', 'types', 'unlisted', 'unresolved'],
  rules              : { cycles: 'error' },
  typescript         : { config: ['site/.vitepress/tsconfig.json'] },
  vitepress          : { entry: ['site/.vitepress/config.ts', 'site/.vitepress/theme/index.ts'] },
  vitest             : { config: ['site/.vitepress/vitest.config.ts'] },
  ignoreDependencies : [
    '@fontsource/fraunces',
    '@fontsource/jetbrains-mono',
    '@fontsource/lora',
    'oxlint'
  ]
})
