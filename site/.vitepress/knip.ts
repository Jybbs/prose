import { parse }        from '@vue/compiler-sfc'
import { defineConfig } from 'knip/config'

const pageScript = (text: string): string => {
  const { descriptor } = parse(text)
  return [descriptor.script?.content, descriptor.scriptSetup?.content].filter(Boolean).join('\n')
}

export default defineConfig({
  compilers  : { md: pageScript },
  include    : ['binaries', 'cycles', 'dependencies', 'duplicates', 'enumMembers', 'exports', 'files', 'namespaceMembers', 'types', 'unlisted', 'unresolved'],
  rules      : { cycles: 'error' },
  workspaces : {
    '.': {
      entry              : ['**/*.md', '.vitepress/**/*.data.ts', '.vitepress/kit/**/*.catalog.tsx', '.vitepress/lib/readme/{cards,record}.ts'],
      project            : ['.vitepress/**/*.{ts,tsx,mjs}', '**/*.md'],
      typescript         : { config: ['.vitepress/tsconfig.json'] },
      vitest             : { config: ['.vitepress/vitest.config.ts'] },
      ignoreDependencies : [
        '@fontsource/fraunces',
        '@fontsource/jetbrains-mono',
        '@fontsource/lora',
        'oxlint'
      ]
    },
    '../tests/site': {
      ignoreUnresolved : ['./pkg/prose_wasm.js'],
      project          : ['**/*.{ts,tsx,mjs}'],
      typescript       : { config: ['../../site/.vitepress/tsconfig.json'] },
      vitest           : { config: ['../../site/.vitepress/vitest.config.ts'] }
    }
  }
})
