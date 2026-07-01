import type { MarkdownRenderer } from '@astrojs/markdown-remark'

import { proseProcessor, shikiConfig } from './config'

// The processor is the non-loader render path. Reads inside a Content Layer
// loader take the loader context's own `renderMarkdown` instead, then pass its
// `html` through `stripParagraph` for the inline shape.
let cachedProcessor: Promise<MarkdownRenderer> | null = null

function processor(): Promise<MarkdownRenderer> {
  return (cachedProcessor ??= proseProcessor.createRenderer({ shikiConfig }))
}

export async function renderMarkdown(markdown: string): Promise<string> {
  const { code } = await (await processor()).render(markdown)
  return code
}

export async function renderInline(markdown: string): Promise<string> {
  return stripParagraph(await renderMarkdown(markdown))
}

// Drops the single paragraph the block renderer wraps an inline field in,
// leaving content that is not one wrapping paragraph untouched.
export function stripParagraph(html: string): string {
  const trimmed = html.trim()
  return trimmed.startsWith('<p>') && trimmed.endsWith('</p>')
    ? trimmed.slice(3, -4)
    : trimmed
}
