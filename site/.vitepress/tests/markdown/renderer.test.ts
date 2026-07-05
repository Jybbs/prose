import * as renderer from '../../lib/markdown/renderer'

describe('renderer', () => {
  it('renders a fenced code block to highlighted HTML', async () => {
    const md = await renderer.getRenderer()
    expect(renderer.renderFencedHtml(md, 'x = 1', 'python')).toContain('<pre')
  })

  it('appends fence meta to the fence line', async () => {
    const md = await renderer.getRenderer()
    expect(renderer.renderFencedHtml(md, 'x = 1', 'python', 'lint=demo-rule/basic')).toContain('<pre')
  })

  it('renders a block field to paragraph HTML', async () => {
    const md = await renderer.getRenderer()
    expect(renderer.renderBlockHtml(md, 'a *b*')).toContain('<p>a <em>b</em></p>')
  })

  it('renders an inline field without a paragraph wrapper', async () => {
    const md = await renderer.getRenderer()
    expect(renderer.renderInlineHtml(md, 'see `x`')).toBe('see <code>x</code>')
  })

  it('replaces an inline field with its rendered counterpart', async () => {
    const md  = await renderer.getRenderer()
    const out = renderer.renderInlineField(md, [{ note: 'see `prose`' }], 'note')
    expect(out[0]).not.toHaveProperty('note')
    expect(out[0].noteHtml).toContain('<code>prose</code>')
  })

  it('renders an array-valued field to an array of HTML strings', async () => {
    const md  = await renderer.getRenderer()
    const out = renderer.renderInlineField(md, [{ tags: ['`a`', '`b`'] }], 'tags')
    expect(out[0].tagsHtml).toEqual(['<code>a</code>', '<code>b</code>'])
  })
})
