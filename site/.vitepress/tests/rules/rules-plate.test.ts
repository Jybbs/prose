// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import RuleTooltipPopper     from '../../theme/components/rules/RuleTooltipPopper.vue'
import RulesPlate            from '../../theme/components/rules/RulesPlate.vue'
import { FAMILY_ORDER }      from '../../lib/shared/registries'
import { rendersAccessibly } from '../axe'
import { popperStubMount }   from '../popper-stub'

vi.mock('../../lib/composables/route', () => ({ useCurrentRule: () => ({ value: null }) }))

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([
    { family: 'alignment', slug: 'align-equals' },
    { family: 'ordering',  slug: 'alphabetize-siblings' },
    { family: 'lint',      slug: 'bare-imports' }
  ]))

const plate = () => mount(RulesPlate, {
  global: { ...popperStubMount, components: { RuleTooltipPopper } }
})

describe('RulesPlate', () => {
  it('sets every family chip once as the key, each linking its family page', () => {
    expect(plate().findAll('.rules-plate-key .family-chip').map(chip => chip.attributes('href')))
      .toStrictEqual(FAMILY_ORDER.map(family => `/rules/${family}/`))
  })

  it('hangs one letter per first letter of a slug, its rules beneath it', () => {
    const letters = plate().findAll('.rules-plate-letter')
    expect(letters.map(letter => [
      letter.get('.rules-plate-cap').text(),
      letter.findAll('a.rules-plate-slug').map(slug => slug.text())
    ])).toStrictEqual([['A', ['align-equals', 'alphabetize-siblings']], ['B', ['bare-imports']]])
  })

  it('leads each slug with its family badge and names the family for a screen reader', () => {
    const item = plate().findAll('.rules-plate-slugs > li')[1]
    expect(item.attributes('data-family')).toBe('ordering')
    expect(item.get('use').attributes('href')).toBe('#nib-harp-small')
    expect(item.get('svg').attributes()).toMatchObject({ 'aria-label': 'Ordering', role: 'img' })
    expect(item.get('a').attributes('href')).toBe('/rules/ordering/alphabetize-siblings')
  })

  rendersAccessibly(() => plate().html())
})
