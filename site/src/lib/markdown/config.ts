import type { AstroMarkdownOptions } from '@astrojs/markdown-remark'

import { SHIKI_THEMES } from '../shared/constants'

// The one Shiki pipeline both render paths share. `astro.config` reads it as
// the site `markdown` config, so the loader-context `renderMarkdown` and the
// standalone processor highlight with the same dual-theme set.
export const markdownConfig: AstroMarkdownOptions = {
  shikiConfig: { themes: SHIKI_THEMES },
}
