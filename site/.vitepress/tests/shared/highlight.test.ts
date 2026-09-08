import { highlight } from '../../lib/shared/highlight'

describe('highlight', () => {
  it.each(['python', 'toml'] as const)('renders %s to dual-theme markup', async lang => {
    const out = await highlight('x = 1', lang)
    expect(out).toContain('<pre')
    expect(out).toContain('--shiki-dark')
  })

  it('carries a decoration through to the rendered span', async () => {
    const out = await highlight('x = 1', 'python', [{
      end        : { character: 1, line: 0 },
      properties : { class: 'lint-flag', 'data-rule': 'demo-rule' },
      start      : { character: 0, line: 0 }
    }])
    expect(out).toContain('lint-flag')
    expect(out).toContain('data-rule="demo-rule"')
  })
})
