// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import TaxonomyChip from '../theme/components/rules/TaxonomyChip.vue'

vi.mock('../lib/composables/route', () => ({ useCurrentRule: () => ({ value: null }) }))

describe('TaxonomyChip', () => {
  it('renders a family chip with the family badge and label', () => {
    const w = mount(TaxonomyChip, { props: { axis: 'family', value: 'alignment' } })
    expect(w.text()).toContain('Alignment')
    expect(w.find('.family-chip-badge').text()).toBe('🪜')
  })

  it('renders a category chip, distinguishing categories from families', () => {
    const w = mount(TaxonomyChip, { props: { axis: 'category', value: 'auto-fix' } })
    expect(w.text()).toContain('Auto-Fix')
    expect(w.find('.category-chip-badge').text()).toBe('A')
  })

  it('renders nothing when the value is unset', () => {
    const w = mount(TaxonomyChip, { props: { axis: 'family' } })
    expect(w.find('.family-chip-badge').exists()).toBe(false)
  })
})
