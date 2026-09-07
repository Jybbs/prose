import * as renderer from '../../lib/markdown/renderer'

describe('renderer', () => {
  it('resolves the one renderer VitePress holds across calls', async () => {
    expect(await renderer.getRenderer()).toBe(await renderer.getRenderer())
  })

  it('renders a fenced code block to highlighted HTML', async () => {
    const md  = await renderer.getRenderer()
    const out = await renderer.renderFencedHtml(md, 'x = 1', 'python')
    expect(out).toContain('<pre')
    expect(out).toContain('<span style=')
  })

  it('appends fence meta to the fence line', async () => {
    const md = await renderer.getRenderer()
    expect(await renderer.renderFencedHtml(md, 'x = 1', 'python', 'lint=demo-rule/basic')).toContain('<pre')
  })

  it('renders a block field to paragraph HTML', async () => {
    const md = await renderer.getRenderer()
    expect(await renderer.renderBlockHtml(md, 'a *b*')).toContain('<p>a <em>b</em></p>')
  })

  it('renders an inline field without a paragraph wrapper', async () => {
    const md = await renderer.getRenderer()
    expect(renderer.renderInlineHtml(md, 'see `x`')).toBe('see <code>x</code>')
  })

  it('replaces an inline field with its walked node tree', async () => {
    const md  = await renderer.getRenderer()
    const out = renderer.inlineNodeField(md, [{ note: 'see `prose`' }], 'note')
    expect(out[0]).not.toHaveProperty('note')
    expect(out[0].noteNodes).toEqual([
      { kind: 'text', text: 'see ' },
      { kind: 'code', text: 'prose' }
    ])
  })

  it('walks an array-valued field to one node tree per entry', async () => {
    const md  = await renderer.getRenderer()
    const out = renderer.inlineNodeField(md, [{ tags: ['`a`', '`b`'] }], 'tags')
    expect(out[0].tagsNodes).toEqual([[{ kind: 'code', text: 'a' }], [{ kind: 'code', text: 'b' }]])
  })

  it('replaces a fenced field with its rendered counterpart', async () => {
    const md  = await renderer.getRenderer()
    const out = await renderer.renderFencedField(md, [{ code: 'x = 1', language: 'python' }], 'code')
    expect(out[0]).not.toHaveProperty('code')
    expect(out[0].codeHtml).toContain('<pre')
  })
})
