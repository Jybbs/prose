import type MarkdownIt from 'markdown-it'

// Holds the word naming each callout kind beside the printer's mark its chip
// sets, a section mark for a tip, a pilcrow for an info, and a dagger for a
// warning or a danger.
const KINDS = {
  danger  : { label: 'Danger',  mark: '†' },
  info    : { label: 'Info',    mark: '¶' },
  tip     : { label: 'Tip',     mark: '§' },
  warning : { label: 'Warning', mark: '†' }
} as const

// Opens each callout on its kind word and the title the page gives it, beside
// a chip holding the kind's mark, which assistive technology skips since the
// word names the kind. VitePress registers the container rules before the
// site's own plugins run, so these renderers replace its opening markup.
export function calloutPlugin(md: MarkdownIt): void {
  for (const [kind, { label, mark }] of Object.entries(KINDS)) {
    md.renderer.rules[`container_${kind}_open`] = (tokens, idx, _options, env) => {
      const token = tokens[idx]!
      const info  = token.info.trim().slice(kind.length).trim()
      const title = info && md.renderInline(info, {
        frontmatter : env.frontmatter,
        references  : env.references
      })
      token.attrJoin('class', `${kind} custom-block`)
      return `<div ${md.renderer.renderAttrs(token).trim()}><p class="custom-block-title">`
           + `<span class="callout-mark" aria-hidden="true">${mark}</span>`
           + `<span class="callout-kind">${label}</span>`
           + (title && `<span class="callout-title">${title}</span>`)
           + '</p>\n'
    }
  }
}
