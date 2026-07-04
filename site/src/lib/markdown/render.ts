import { proseProcessor, shikiConfig } from './config'
import { lazy }                        from '../shared/lazy'

// The processor is the non-loader render path, whereas reads inside a Content
// Layer loader take the loader context's own `renderMarkdown`.
const processor = lazy(() => proseProcessor.createRenderer({ shikiConfig }))

export async function renderBlock(markdown: string): Promise<string> {
  const { code } = await (await processor()).render(markdown)
  return code
}

export async function renderInline(markdown: string): Promise<string> {
  return stripParagraph(await renderBlock(markdown))
}

// Drops the single paragraph the block renderer wraps an inline field in,
// leaving content that is not one wrapping paragraph untouched.
function stripParagraph(html: string): string {
  const trimmed = html.trim()
  return trimmed.startsWith('<p>') && trimmed.endsWith('</p>')
    ? trimmed.slice(3, -4)
    : trimmed
}

if (import.meta.vitest) {
  const { describe, expect, test } = import.meta.vitest

  describe('renderBlock', () => {
    test('renders a paragraph', async () => {
      expect(await renderBlock('Hello there reader')).toContain('<p>Hello there reader</p>')
    })

    test('runs the body-mark plugins', async () => {
      expect(await renderBlock('Prose reads well')).toContain('prose-mark')
    })
  })

  describe('renderInline', () => {
    test('strips the single wrapping paragraph', async () => {
      expect(await renderInline('Hello there reader')).toBe('Hello there reader')
    })

    test('leaves non-paragraph output untouched', async () => {
      expect(await renderInline('- item')).toContain('<ul>')
    })
  })
}
