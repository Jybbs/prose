// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import FamilyChip from '../../theme/components/rules/FamilyChip.vue'

const current = vi.hoisted(() => ({ value: null as { family: string } | null }))

vi.mock('../../lib/composables/route', () => ({ useCurrentRule: () => current }))

describe('FamilyChip', () => {
  afterEach(() => { current.value = null })

  it('sets the family badge on the tint beside the label, linked to the family page', () => {
    const chip = mount(FamilyChip, { props: { family: 'alignment' } })
    expect(chip.element.tagName).toBe('A')
    expect(chip.attributes()).toMatchObject({ 'data-family': 'alignment', href: '/rules/alignment/' })
    expect(chip.classes().toSorted()).toStrictEqual(['family-chip', 'nib-ground'])
    expect(chip.find('use').attributes('href')).toBe('#nib-ladder-small')
    expect(chip.text()).toBe('Alignment')
  })

  it('links no page for a glossary family that holds no rules', () => {
    const chip = mount(FamilyChip, { props: { family: 'engine' } })
    expect(chip.element.tagName).toBe('SPAN')
    expect(chip.attributes('href')).toBeUndefined()
    expect(chip.text()).toBe('Engine')
  })

  it('links no page where the chip is told not to', () => {
    expect(mount(FamilyChip, { props: { family: 'lint', linked: false } }).attributes('href'))
      .toBeUndefined()
  })

  it('reads the current rule page\'s family where none is given', () => {
    current.value = { family: 'ordering' }
    expect(mount(FamilyChip).attributes('href')).toBe('/rules/ordering/')
  })

  it('renders nothing off a rule page with no family given', () => {
    expect(mount(FamilyChip).html()).toBe('<!--v-if-->')
  })
})
