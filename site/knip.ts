import { parse }           from '@vue/compiler-sfc'
import type { KnipConfig } from 'knip'

const pageScript = (text: string): string => {
  const { descriptor } = parse(text)
  return [descriptor.script?.content, descriptor.scriptSetup?.content].filter(Boolean).join('\n')
}

const config: KnipConfig = {
  entry     : ['**/*.md', '.vitepress/**/*.data.ts', '.vitepress/lib/og/render/resvg-worker.mjs'],
  project   : ['.vitepress/**/*.{ts,vue,mjs}'],
  ignoreDependencies: [
    '@fontsource/fraunces',
    '@fontsource/jetbrains-mono',
    '@fontsource/lora',
    '@shikijs/types',
    'oxlint',
    'vue-tsc'
  ],
  compilers : { md: pageScript }
}

export default config
