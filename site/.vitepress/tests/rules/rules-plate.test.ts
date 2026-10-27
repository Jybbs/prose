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
  it('sets every family once as the key, each a button that pins its family', () => {
    const chips = plate().findAll('.rules-plate-key .family-chip').map(chip =>
      [chip.element.tagName, chip.attributes('data-family'), chip.attributes('aria-pressed')])
    expect(chips).toStrictEqual(FAMILY_ORDER.map(family => ['BUTTON', family, 'false']))
  })

  it('raises the family a reader points at in the key until the pointer leaves', async () => {
    const w    = plate()
    const chip = w.get('.rules-plate-key [data-family="ordering"]')
    await chip.trigger('mouseenter')
    expect(w.get('.rules-plate').attributes('data-raised')).toBe('ordering')
    expect(w.findAll('.rules-plate-slugs > li.is-raised').map(item => item.text()))
      .toStrictEqual(['alphabetize-siblings'])
    await chip.trigger('mouseleave')
    expect(w.get('.rules-plate').attributes('data-raised')).toBeUndefined()
  })

  it('holds a clicked family raised until a second click or Escape releases it', async () => {
    const w    = plate()
    const chip = w.get('.rules-plate-key [data-family="lint"]')
    await chip.trigger('click')
    expect(chip.attributes('aria-pressed')).toBe('true')
    expect(w.get('.rules-plate').attributes('data-raised')).toBe('lint')
    await chip.trigger('click')
    expect(w.get('.rules-plate').attributes('data-raised')).toBeUndefined()
    await w.get('.rules-plate-slugs > li[data-family="alignment"]').trigger('click')
    expect(w.get('.rules-plate').attributes('data-raised')).toBe('alignment')
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await w.vm.$nextTick()
    expect(w.get('.rules-plate').attributes('data-raised')).toBeUndefined()
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
