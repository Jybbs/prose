// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import RuleTooltipPopup      from '../../theme/components/rules/RuleTooltipPopup.vue'
import RulesPlate            from '../../theme/components/rules/RulesPlate.vue'
import { FAMILY_ORDER }      from '../../lib/shared/registries'
import { rendersAccessibly } from '../axe'
import { domTest }           from '../dom'

vi.mock('../../lib/composables/route', () => ({ useCurrentRule: () => ({ value: null }) }))

vi.mock('../../lib/rules/rules.data', async () =>
  (await import('../rules-data-stub')).rulesDataStub([
    { family: 'alignment', slug: 'align-equals' },
    { family: 'ordering',  slug: 'alphabetize-siblings' },
    { family: 'lint',      slug: 'bare-imports' }
  ]))

const plate = () => mount(RulesPlate, {
  attachTo : document.body,
  global   : { components: { RuleTooltipPopup } }
})

const letters = (w: ReturnType<typeof plate>) => w.findAll('.rules-plate-letter:not(.is-out)').map(letter => [
  letter.attributes('data-letter'),
  letter.findAll('a.rules-plate-slug').map(slug => slug.text())
])

describe('RulesPlate', () => {
  it('sets the key as a toolbar of toggles, each joined to an arrow linking its family\'s page', () => {
    const w = plate()
    expect(w.get('.rules-plate-key').attributes()).toMatchObject({ 'aria-label': 'Rule families', role: 'toolbar' })
    expect(w.findAll('.rules-plate-split').map(split => [
      split.get('button').attributes('aria-pressed'),
      split.get('a.rules-plate-go').attributes('href')
    ])).toStrictEqual(FAMILY_ORDER.map(family => ['false', `/rules/${family}/`]))
    expect(w.findAll('.rules-plate-key button').map(chip => chip.attributes('tabindex')))
      .toStrictEqual(FAMILY_ORDER.map((_, at) => at === 0 ? '0' : '-1'))
  })

  it('draws one census segment per family as wide as its share of the list', () => {
    const segments = plate().findAll('.rules-plate-census > span')
    expect(segments.map(segment => [segment.attributes('data-family'), segment.attributes('style')]))
      .toStrictEqual(FAMILY_ORDER.map(family =>
        [family, `flex-grow: ${['alignment', 'lint', 'ordering'].includes(family) ? 1 : 0}; flex-shrink: 1; flex-basis: 0px;`]))
  })

  it('raises the segment and names the count of the family a reader points at', async () => {
    const w    = plate()
    const chip = w.get('.rules-plate-split[data-family="ordering"] button')
    await chip.trigger('mouseenter')
    expect(w.get('.rules-plate-census [data-family="ordering"]').classes()).toContain('is-raised')
    expect(w.get('.rules-plate-census-cap').text()).toBe('1 of 3 rules')
    await chip.trigger('mouseleave')
    expect(w.get('.rules-plate-census-cap').text()).toBe('3 rules')
  })

  domTest('gathers a pressed family\'s rules above the letters until a second press returns them', async ({ reducedMotion }) => {
    reducedMotion(true)
    const w    = plate()
    const chip = w.get('.rules-plate-split[data-family="ordering"] button')
    await chip.trigger('click')
    await w.vm.$nextTick()
    expect(chip.attributes('aria-pressed')).toBe('true')
    expect(letters(w)).toStrictEqual([['gathered', ['alphabetize-siblings']], ['a', ['align-equals']], ['b', ['bare-imports']]])
    expect(w.get('.rules-plate-gathered .rules-plate-cap').attributes('data-family')).toBe('ordering')
    expect(w.get('.rules-plate-live').text()).toBe('1 Ordering rules moved above the list.')
    expect(w.get('.rules-plate-all').classes()).not.toContain('is-idle')
    await chip.trigger('click')
    await w.vm.$nextTick()
    expect(letters(w)).toStrictEqual([['a', ['align-equals', 'alphabetize-siblings']], ['b', ['bare-imports']]])
    expect(w.get('.rules-plate-live').text()).toBe('All families shown.')
  })

  domTest('hides a letter the gathering empties and returns every row under Escape or All families', async ({ reducedMotion }) => {
    reducedMotion(true)
    const w = plate()
    await w.get('.rules-plate-split[data-family="lint"] button').trigger('click')
    await w.vm.$nextTick()
    expect(w.get('[data-letter="b"]').classes()).toContain('is-out')
    window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await w.vm.$nextTick()
    await w.vm.$nextTick()
    expect(w.find('.rules-plate-gathered').exists()).toBe(false)
    await w.get('.rules-plate-split[data-family="lint"] button').trigger('click')
    await w.get('.rules-plate-all').trigger('click')
    await w.vm.$nextTick()
    await w.vm.$nextTick()
    expect(w.find('.rules-plate-gathered').exists()).toBe(false)
    expect(document.activeElement).toBe(w.get('.rules-plate-split[data-family="lint"] button').element)
  })

  it('leaves the arrangement as it stands while a reader points down the list', async () => {
    const w = plate()
    await w.get('.rules-plate-slugs > li[data-slug="bare-imports"]').trigger('mouseenter')
    expect(w.find('.rules-plate-gathered').exists()).toBe(false)
    expect(w.get('.rules-plate-census-cap').text()).toBe('3 rules')
  })

  it('moves the focus across the toolbar under the arrow keys, Home, and End, wrapping past either end', async () => {
    const w     = plate()
    const chips = w.findAll('.rules-plate-key button')
    await chips[0]!.trigger('keydown', { key: 'ArrowLeft' })
    expect(document.activeElement).toBe(chips.at(-1)!.element)
    expect(chips.at(-1)!.attributes('tabindex')).toBe('0')
    await chips.at(-1)!.trigger('keydown', { key: 'Home' })
    expect(document.activeElement).toBe(chips[0]!.element)
    await chips[0]!.trigger('keydown', { key: 'Enter' })
    expect(document.activeElement).toBe(chips[0]!.element)
  })

  it('leads each slug with its family badge and names the family for a screen reader', () => {
    const item = plate().get('.rules-plate-slugs > li[data-slug="alphabetize-siblings"]')
    expect(item.attributes('data-family')).toBe('ordering')
    expect(item.get('use').attributes('href')).toBe('#nib-harp-small')
    expect(item.get('svg').attributes()).toMatchObject({ 'aria-label': 'Ordering', role: 'img' })
    expect(item.get('a').attributes('href')).toBe('/rules/ordering/alphabetize-siblings')
  })

  rendersAccessibly(() => plate().html())
})
