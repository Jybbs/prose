import { defineEcConfig } from '@astrojs/starlight/expressive-code'

import { lintFlagPlugin } from './src/lib/markdown/config'
import { SHIKI_THEMES }   from './src/lib/shared/constants'

export default defineEcConfig({
  plugins                   : [lintFlagPlugin],
  themes                    : [SHIKI_THEMES.dark, SHIKI_THEMES.light],
  useStarlightUiThemeColors : true
})
