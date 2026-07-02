import { defineEcConfig } from '@astrojs/starlight/expressive-code'

import { lintFlagPlugin } from './src/lib/markdown/config'
import { SHIKI_THEMES }   from './src/lib/shared/constants'

// The `<Code>` component re-creates the Expressive Code renderer from a
// serialized config, which an inline `astro.config.ts` options object
// carrying a plugin function cannot survive, so the options live here.
export default defineEcConfig({
  plugins                   : [lintFlagPlugin],
  themes                    : [SHIKI_THEMES.dark, SHIKI_THEMES.light],
  useStarlightUiThemeColors : true
})
