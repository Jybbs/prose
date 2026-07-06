// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import ScopeSpecimen        from '../../theme/components/suppression/ScopeSpecimen.vue'
import { expectAccessible } from '../axe'

vi.mock('../../lib/suppression/directives.data', () => ({
  data: [
    { form : '# prose: off',              id : 'prose-off',          scope : 'file'  },
    { form : '# fmt: off',                id : 'fmt-off',            scope : 'block',
      pairId : 'fmt-on', pairRole : 'opens' },
    { form : '# fmt: on',                 id : 'fmt-on',             scope : 'block',
      pairId : 'fmt-off', pairRole : 'closes' },
    { form : '# fmt: skip',               id : 'fmt-skip',           scope : 'line'  },
    { form : '# prose: skip[<rule>, ...]', id : 'prose-skip-rules',  scope : 'line'  },
    { form : '# prose: ignore[<rule>, ...]', id : 'prose-ignore-rules', scope : 'line' },
    { form : '# prose: keep',             id : 'prose-keep',         scope : 'dict'  }
  ]
}))

describe('ScopeSpecimen', () => {
  it('renders one legend band per scope in the shared order', () => {
    const rows = mount(ScopeSpecimen).findAll('.scope-specimen-legend-row')
    expect(rows.map(r => r.attributes('data-scope'))).toEqual(['file', 'block', 'line', 'dict'])
  })

  it('composes the paired display form from the registry', () => {
    const labels = mount(ScopeSpecimen).findAll('.scope-specimen-legend-directive').map(d => d.text())
    expect(labels).toContain('# fmt: off … # fmt: on')
    expect(labels).toContain('# prose: skip[<rule>, ...]')
  })

  it('derives each legend href from the directive scope', () => {
    const links = mount(ScopeSpecimen).findAll('.scope-specimen-legend-directive a')
    expect(links.map(a => a.attributes('href'))).toEqual([
      '/reference/suppression-directives#file-level-suppression',
      '/reference/suppression-directives#block-markers',
      '/reference/suppression-directives#line-markers',
      '/reference/suppression-directives#line-markers',
      '/reference/suppression-directives#line-markers',
      '/reference/suppression-directives#dict-literal-order-preservation'
    ])
  })

  it('renders with no axe violations', async () => {
    await expectAccessible(mount(ScopeSpecimen).html())
  })
})
