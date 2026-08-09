import type { DecorationItem } from '@shikijs/types'

import { codeHighlighter } from '../markdown/highlighter'
import { SHIKI_THEMES }    from './constants'

// Highlights a code string to dual-theme HTML through the shared client
// highlighter, carrying optional shiki decorations so a runtime surface
// paints the same lint squiggles the fixture pages render.
export async function highlight(
  code         : string,
  lang         : 'python' | 'toml',
  decorations ?: readonly DecorationItem[]
): Promise<string> {
  const highlighter = await codeHighlighter()
  return highlighter.codeToHtml(code, {
    decorations : decorations ? [...decorations] : [],
    lang,
    themes      : SHIKI_THEMES
  })
}
