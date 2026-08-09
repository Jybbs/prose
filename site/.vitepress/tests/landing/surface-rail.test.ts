// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'
import { nextTick }             from 'vue'

import type { RenderedRule }   from '../../lib/rules/rules.data'
import SurfaceRail             from '../../theme/components/landing/surfaces/SurfaceRail.vue'
import { domTest, nextFrame }  from '../dom'
import { popperStubMount }     from '../popper-stub'

// The mock keys each measurement off the element it observes rather than off
// call order, and drives the pointer the edge glide reads.
const { pointer, widths } = vi.hoisted(() => ({
  pointer : { elementX: 0, isOutside: true },
  widths  : { rail: 0, window: 0 }
}))

vi.mock('@vueuse/core', async importOriginal => {
  const actual   = await importOriginal<typeof import('@vueuse/core')>()
  const { computed, ref: vueRef } = await import('vue')
  return {
    ...actual,
    useElementSize: (target: Parameters<typeof actual.useElementSize>[0]) => ({
      height : vueRef(0),
      width  : computed(() => {
        const el = actual.unrefElement(target as never) as HTMLElement | undefined
        return el?.classList.contains('surface-rail-window') ? widths.window : widths.rail
      })
    }),
    useMouseInElement: () => ({
      elementX  : computed(() => pointer.elementX),
      isOutside : computed(() => pointer.isOutside)
    })
  }
})

const rule = (slug: string): RenderedRule => ({
  family      : 'alignment',
  familyBadge : '🪜',
  href        : `/rules/alignment/${slug}`,
  slug
} as RenderedRule)

const RULES = [
  'align-equals', 'align-colons', 'align-commas', 'align-comments', 'align-arrows'
].map(rule)

const mountRail = async (rules: readonly RenderedRule[] = RULES) => {
  const wrapper = mount(SurfaceRail, { global: popperStubMount, props: { rules } })
  await flushPromises()
  return wrapper
}

describe('SurfaceRail', () => {
  beforeEach(() => {
    pointer.elementX = 0
    pointer.isOutside = true
    widths.rail = 0
    widths.window = 0
  })

  domTest('renders one pip per rule, numbered from one and linked out', async ({ reducedMotion }) => {
    reducedMotion(true)
    const pips = (await mountRail()).findAll('.surface-pip')

    expect(pips).toHaveLength(RULES.length)
    expect(pips.map(p => p.text())).toEqual(['01', '02', '03', '04', '05'])
    expect(pips[0].attributes('href')).toBe('/rules/alignment/align-equals')
    expect(pips[0].attributes('aria-label')).toBe('align-equals')
  })

  domTest('names the first rule before the pointer has picked one', async ({ reducedMotion }) => {
    reducedMotion(true)
    const wrapper = await mountRail()

    expect(wrapper.get('.surface-rail-chip').text()).toContain('align-equals')
    expect(wrapper.findAll('.surface-pip.active')).toHaveLength(0)
  })

  domTest('holds every pip in the scrolling window while the roster fits', async ({ reducedMotion }) => {
    reducedMotion(true)
    widths.rail   = 400
    widths.window = 400
    const wrapper = await mountRail()

    expect(wrapper.get('.surface-rail-row').attributes('data-bookends')).toBe('false')
    expect(wrapper.findAll('.surface-rail-end')).toHaveLength(0)
    expect(wrapper.findAll('.surface-rail-track .surface-pip')).toHaveLength(RULES.length)
    expect(wrapper.findAll('.surface-rail-chevron')).toHaveLength(0)
  })

  domTest('pins the first and last rules outside the window on overrun', async ({ reducedMotion }) => {
    reducedMotion(true)
    widths.rail = 60
    const wrapper = await mountRail()

    const ends = wrapper.findAll('.surface-rail-end')
    expect(wrapper.get('.surface-rail-row').attributes('data-bookends')).toBe('true')
    expect(ends.map(e => e.attributes('aria-label'))).toEqual(['align-equals', 'align-arrows'])
    expect(wrapper.findAll('.surface-rail-track .surface-pip').map(p => p.text()))
      .toEqual(['02', '03', '04'])
  })

  domTest('circles a hovered pip and holds it once the pointer leaves', async ({ reducedMotion }) => {
    reducedMotion(true)
    const wrapper = await mountRail()
    const pips    = wrapper.findAll('.surface-pip')

    await pips[2].trigger('mouseenter')
    expect(pips[2].classes()).toContain('active')
    expect(wrapper.get('.surface-rail-chip').text()).toContain('align-commas')

    await wrapper.trigger('mouseleave')
    expect(wrapper.findAll('.surface-pip.active').map(p => p.text())).toEqual(['03'])
  })

  domTest('counts each pip away from the circled one', async ({ reducedMotion }) => {
    reducedMotion(true)
    const wrapper = await mountRail()
    const pips    = wrapper.findAll('.surface-pip')

    await pips[3].trigger('focus')
    expect(pips.map(p => p.attributes('style'))).toEqual([
      '--d: 3;', '--d: 2;', '--d: 1;', '--d: 0;', '--d: 1;'
    ])
  })

  domTest('swaps the name forward or back with the direction of travel', async ({ reducedMotion }) => {
    reducedMotion(true)
    const wrapper = await mountRail()
    const pips    = wrapper.findAll('.surface-pip')

    await pips[3].trigger('mouseenter')
    expect(wrapper.findComponent({ name: 'SurfaceRailName' }).props('swap')).toBe('surface-rail-fwd')

    await pips[1].trigger('mouseenter')
    expect(wrapper.findComponent({ name: 'SurfaceRailName' }).props('swap')).toBe('surface-rail-back')
  })

  domTest('offers a chevron to each limit only while the window travels', async ({ reducedMotion }) => {
    reducedMotion(true)
    widths.rail   = 60
    widths.window = 20
    const wrapper = await mountRail()

    const chevrons = wrapper.findAll('.surface-rail-chevron')
    expect(chevrons.map(c => c.attributes('aria-label')))
      .toEqual(['Travel to the first rule', 'Travel to the last rule'])
    expect(chevrons[0].attributes('disabled')).toBeDefined()
    expect(chevrons[1].attributes('disabled')).toBeUndefined()
  })

  domTest('travels the window to its far end on the closing chevron', async ({ reducedMotion }) => {
    reducedMotion(true)
    widths.rail   = 60
    widths.window = 20
    const wrapper = await mountRail()
    const win     = wrapper.get('.surface-rail-window').element as HTMLElement

    Object.defineProperty(win, 'scrollWidth', { configurable: true, value: 500 })
    const scrollTo = vi.spyOn(win, 'scrollTo').mockImplementation(() => {})

    await wrapper.findAll('.surface-rail-chevron')[1].trigger('click')
    expect(scrollTo).toHaveBeenCalledWith({ behavior: 'auto', left: 500 })
  })

  domTest('picks the rule under a pointer the glide carries the strip past', async ({ reducedMotion }) => {
    reducedMotion(true)
    widths.rail       = 60
    widths.window     = 20
    pointer.isOutside = false
    pointer.elementX  = 1
    const wrapper     = await mountRail()

    await nextTick()
    await nextFrame()
    await nextTick()

    expect(wrapper.get('.surface-rail-window').attributes('data-edge')).toBe('end')
  })
})
