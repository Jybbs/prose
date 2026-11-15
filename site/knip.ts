import { parse }        from '@vue/compiler-sfc'
import { defineConfig } from 'knip/config'

const pageScript = (text: string): string => {
  const { descriptor } = parse(text)
  return [descriptor.script?.content, descriptor.scriptSetup?.content].filter(Boolean).join('\n')
}

export default defineConfig({
  entry              : ['**/*.md', '.vitepress/**/*.data.ts', '.vitepress/kit/**/*.catalog.tsx', '.vitepress/lib/readme/{cards,record}.ts'],
  project            : ['.vitepress/**/*.{ts,tsx,mjs}', 'prototypes/**/*.{ts,tsx}', '**/*.md'],
  compilers          : { md: pageScript },
  include            : ['binaries', 'cycles', 'dependencies', 'duplicates', 'enumMembers', 'exports', 'files', 'namespaceMembers', 'types', 'unlisted', 'unresolved'],
  rules              : { cycles: 'error' },
  ignoreDependencies : [
    '@fontsource/fraunces',
    '@fontsource/jetbrains-mono',
    '@fontsource/lora',
    'oxlint'
  ]
})
