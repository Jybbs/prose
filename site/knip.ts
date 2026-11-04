import { parse }        from '@vue/compiler-sfc'
import { defineConfig } from 'knip/config'

const pageScript = (text: string): string => {
  const { descriptor } = parse(text)
  return [descriptor.script?.content, descriptor.scriptSetup?.content].filter(Boolean).join('\n')
}

export default defineConfig({
  entry              : ['**/*.md', '.vitepress/**/*.data.ts'],
  project            : ['.vitepress/**/*.{ts,vue,mjs}', 'prototypes/**/*.{ts,vue}', '**/*.md'],
  compilers          : { md: pageScript },
  ignoreIssues       : { '.vitepress/lib/shared/nib.ts': ['exports'] },
  include            : ['binaries', 'cycles', 'dependencies', 'duplicates', 'enumMembers', 'exports', 'files', 'namespaceMembers', 'types', 'unlisted', 'unresolved'],
  rules              : { cycles: 'error' },
  ignoreDependencies : [
    '@fontsource/fraunces',
    '@fontsource/jetbrains-mono',
    '@fontsource/lora',
    'oxlint'
  ]
})
