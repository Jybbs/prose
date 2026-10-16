import type MarkdownIt from 'markdown-it'

import { indexSectionOf } from '../shared/registries'

// Inserts a section index page's H1 ahead of the inline parse, which reads the
// page's `SECTIONS` label as the heading's text, so the anchor and title
// plugins handle the heading as though the page had typed it. A page whose
// frontmatter sets `heading: false` takes no heading, and an index page that
// types its own H1 fails the build.
export function sectionHeadingPlugin(md: MarkdownIt): void {
  md.core.ruler.after('block', 'section-heading', state => {
    const section = indexSectionOf(state.env.relativePath)
    if (section === undefined || state.env.frontmatter?.heading === false) return
    if (state.tokens.some(token => token.type === 'heading_open' && token.tag === 'h1')) {
      throw new Error(`${state.env.relativePath} types its own H1, which SECTIONS supplies`)
    }
    const open   = new state.Token('heading_open', 'h1', 1)
    const inline = new state.Token('inline', '', 0)
    const close  = new state.Token('heading_close', 'h1', -1)
    for (const token of [open, inline, close]) token.block = true
    open.markup     = '#'
    close.markup    = '#'
    inline.children = []
    inline.content  = section.label
    state.tokens.unshift(open, inline, close)
  })
}
