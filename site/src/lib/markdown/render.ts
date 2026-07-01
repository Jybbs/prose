import type { MarkdownRenderer } from '@astrojs/markdown-remark'

import { proseProcessor, shikiConfig } from './config'

// The processor is the non-loader render path, whereas reads inside a Content
// Layer loader take the loader context's own `renderMarkdown`.
let cachedProcessor: Promise<MarkdownRenderer> | null = null

function processor(): Promise<MarkdownRenderer> {
  return (cachedProcessor ??= proseProcessor.createRenderer({ shikiConfig }))
}

export async function renderInline(markdown: string): Promise<string> {
  const { code } = await (await processor()).render(markdown)
  return stripParagraph(code)
}

// Drops the single paragraph the block renderer wraps an inline field in,
// leaving content that is not one wrapping paragraph untouched.
function stripParagraph(html: string): string {
  const trimmed = html.trim()
  return trimmed.startsWith('<p>') && trimmed.endsWith('</p>')
    ? trimmed.slice(3, -4)
    : trimmed
}
