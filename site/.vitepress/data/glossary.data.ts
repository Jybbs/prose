import { defineLoader } from 'vitepress'

import { glossary }    from '../lib/glossary/glossary'
import { getRenderer } from '../lib/markdown/renderer'

export interface RenderedGlossaryEntry {
  aliases        : readonly string[]
  definitionHtml : string
  href          ?: string
  initial        : string
  slug           : string
  tooltipHtml    : string
}

export interface GlossaryData {
  entries: Record<string, RenderedGlossaryEntry>
}

declare const data: GlossaryData
export { data }

export default defineLoader({
  watch: [],
  async load(): Promise<GlossaryData> {
    const md      = await getRenderer()
    const entries : Record<string, RenderedGlossaryEntry> = {}

    for (const [slug, entry] of Object.entries(glossary)) {
      const definitionHtml = md.renderInline(entry.definition)
      const parts          = [
        `<div class="glossary-tooltip-title">${md.utils.escapeHtml(slug)}</div>`,
        `<div class="glossary-tooltip-divider" aria-hidden="true"></div>`,
        `<div class="glossary-tooltip-body">${definitionHtml}</div>`
      ]
      if (entry.href) {
        parts.push(`<a href="${md.utils.escapeHtml(entry.href)}" class="glossary-tooltip-link">Read more →</a>`)
      }
      entries[slug] = {
        aliases     : entry.aliases ?? [],
        definitionHtml,
        href        : entry.href,
        initial     : firstLetter(slug),
        slug,
        tooltipHtml : parts.join('')
      }
    }

    return { entries }
  }
})

function firstLetter(slug: string): string {
  return Iterator.from(slug).find(ch => /[A-Za-z]/.test(ch))?.toUpperCase() ?? '#'
}
