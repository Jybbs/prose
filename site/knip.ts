import { parse }        from '@vue/compiler-sfc'
import { defineConfig } from 'knip/config'

const pageScript = (text: string): string => {
  const { descriptor } = parse(text)
  return [descriptor.script?.content, descriptor.scriptSetup?.content].filter(Boolean).join('\n')
}

export default defineConfig({
  entry              : ['**/*.md', '.vitepress/**/*.data.ts'],
  project            : ['.vitepress/**/*.{ts,vue,mjs}'],
  compilers          : { md: pageScript },
  ignoreDependencies : [
    '@fontsource/fraunces',
    '@fontsource/jetbrains-mono',
    '@fontsource/lora',
    'oxlint',
    'vue-tsc'
  ]
})
