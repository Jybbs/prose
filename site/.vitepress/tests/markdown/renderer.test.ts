import { getRenderer, renderFencedHtml, renderInlineField } from '../../lib/markdown/renderer'

describe('renderer', () => {
  it('renders a fenced code block to highlighted HTML', async () => {
    const md = await getRenderer()
    expect(renderFencedHtml(md, 'x = 1', 'python')).toContain('<pre')
  })

  it('replaces an inline field with its rendered counterpart', async () => {
    const md  = await getRenderer()
    const out = renderInlineField(md, [{ note: 'see `prose`' }], 'note')
    expect(out[0]).not.toHaveProperty('note')
    expect(out[0].noteHtml).toContain('<code>prose</code>')
  })

  it('renders an array-valued field to an array of HTML strings', async () => {
    const md  = await getRenderer()
    const out = renderInlineField(md, [{ tags: ['`a`', '`b`'] }], 'tags')
    expect(out[0].tagsHtml).toEqual(['<code>a</code>', '<code>b</code>'])
  })
})
