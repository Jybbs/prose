// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import { directiveHref, SCOPE_ORDER } from '../../lib/suppression/scopes'
import ScopeSpecimen                  from '../../theme/components/suppression/ScopeSpecimen.vue'
import { expectAccessible }           from '../axe'

vi.mock('../../lib/suppression/directives.data', async () =>
  ({ data: (await import('../../lib/suppression/directives')).DIRECTIVES }))

describe('ScopeSpecimen', () => {
  it('renders one legend band per scope in the shared order', () => {
    const rows = mount(ScopeSpecimen).findAll('.scope-specimen-legend-row')
    expect(rows.map(r => r.attributes('data-scope'))).toStrictEqual([...SCOPE_ORDER])
  })

  it('composes the paired display form from the registry', () => {
    const labels = mount(ScopeSpecimen).findAll('.scope-specimen-legend-directive').map(d => d.text())
    expect(labels).toContain('# fmt: off … # fmt: on')
    expect(labels).toContain('# prose: skip[<rule>, ...]')
  })

  it('derives each legend href from the directive scope', () => {
    const links  = mount(ScopeSpecimen).findAll('.scope-specimen-legend-directive a')
    const scopes = ['file', 'block', 'line', 'line', 'line', 'construct'] as const
    expect(links.map(a => a.attributes('href'))).toStrictEqual(scopes.map(scope => directiveHref(scope)))
  })

  it('brackets the skipped statement across every line it spans', () => {
    const lines = mount(ScopeSpecimen).findAll('.scope-specimen-line[data-scope="line"]')
    expect(lines.map(l => l.attributes('data-bracket'))).toStrictEqual(['open', 'mid', 'close', 'solo'])
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mount(ScopeSpecimen).html())
  })
})
