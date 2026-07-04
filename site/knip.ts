import type { KnipConfig } from 'knip'

const config: KnipConfig = {
  entry              : ['src/lib/landing/typing-demo-buffer.ts', 'src/lib/landing/typing-state-machine.ts'],
  ignore             : ['tests/fixtures/**'],
  ignoreDependencies : [/^@fontsource/, /^@iconify-json/, '@astrojs/check', 'oxlint']
}

export default config
