// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'

import { IdentityBadges }                     from '../../theme/components/identity/IdentityBadges'
import { IdentityMarks }                      from '../../theme/components/identity/IdentityMarks'
import { IdentityPalette }                    from '../../theme/components/identity/IdentityPalette'
import { IdentityRoles }                      from '../../theme/components/identity/IdentityRoles'
import { IdentityTypefaces }                  from '../../theme/components/identity/IdentityTypefaces'
import loader, { data }                       from '../../lib/identity/identity.data'
import { Slider }                             from '../../kit'
import { LEVELS, PASSAGE_TOKENS }             from '../../lib/identity/roles'
import { familyEmblem, strokesOf }            from '../../lib/shared/emblems'
import { FAMILY_ORDER, GLOSSARY_FAMILY_META } from '../../lib/shared/registries'

vi.mock('../../lib/identity/identity.data', async importOriginal => {
  const actual = await importOriginal<typeof import('../../lib/identity/identity.data')>()
  return { ...actual, data: await actual.default.load([]) }
})

describe('the identity data', () => {
  it('resolves every role and every passage token to a color in each scheme', () => {
    expect(loader.watch).toBeDefined()
    const values = [...data.roles.flatMap(role => Object.values(role.value)), ...Object.values(data.passage).flatMap(Object.values)]
    expect(values.filter(value => !/^#[\da-f]{6}$/.test(value))).toStrictEqual([])
    expect(Object.keys(data.passage.light)).toStrictEqual(expect.arrayContaining([...PASSAGE_TOKENS]))
  })

  it('reads a mark role for each callout kind and judges each ground by the ink it carries', () => {
    expect(data.roles.filter(role => role.group === 'marks').map(role => role.id))
      .toStrictEqual(['callout-tip', 'callout-info', 'callout-warning', 'callout-danger'])
    const page = data.roles.find(role => role.id === 'page')!
    expect(page.contrast.light.ratio).toBe(data.roles.find(role => role.id === 'text-1')!.contrast.light.ratio)
  })

  it('reads a nib ink in each scheme for every family and a rule count for every rule family', () => {
    expect(Object.keys(data.inks).toSorted()).toStrictEqual(Object.keys(GLOSSARY_FAMILY_META).toSorted())
    expect(Object.keys(data.ruleCounts).toSorted()).toStrictEqual([...FAMILY_ORDER].toSorted())
  })
})

describe('IdentityPalette', () => {
  it('names the strip the reader pins in the entry under the band until Escape releases it', async () => {
    const wrapper = mount(IdentityPalette, { attachTo: document.body })
    const strips  = wrapper.findAll('.identity-strip')
    expect(strips).toHaveLength(data.swatches.length)
    const last = data.swatches.at(-1)!
    await strips.find(strip => strip.attributes('aria-label') === `${last.title}, ${last.hex}`)!.trigger('click')
    await flushPromises()
    expect(wrapper.text()).toContain(`Pinned · ${last.title}`)
    expect(wrapper.get('.identity-entry').text()).toContain(last.hex)
    expect(wrapper.findAll('.identity-row-summary').map(row => row.attributes('aria-expanded'))).toStrictEqual(['true', 'false', 'false'])
    await wrapper.findAll('.identity-row-summary')[1]!.trigger('click')
    expect(wrapper.get('.identity-entry').text()).toContain(`${last.contrast.light.ratio.toFixed(2)}:1`)
    globalThis.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape' }))
    await flushPromises()
    expect(wrapper.text()).not.toContain('Pinned ·')
    wrapper.unmount()
  })
})

describe('IdentityRoles', () => {
  it('sets the passage in the chosen scheme and marks each place it uses the role the reader points at', async () => {
    const wrapper = mount(IdentityRoles)
    const passage = () => wrapper.get('[data-role="page"]').attributes('style')
    expect(passage()).toContain(`--vp-c-bg: ${data.passage.light['--vp-c-bg']}`)
    await wrapper.findAll('.toggle-pill').find(pill => pill.text() === 'Dark')!.trigger('click')
    expect(passage()).toContain(`--vp-c-bg: ${data.passage.dark['--vp-c-bg']}`)
    const legend = wrapper.findAll('.identity-legend')
    expect(legend).toHaveLength(data.roles.length)
    await legend[data.roles.findIndex(role => role.id === 'text-1')]!.trigger('mouseenter')
    const marked = wrapper.findAll('[data-marked]')
    expect(marked.length).toBeGreaterThan(1)
    expect(new Set(marked.map(each => each.attributes('data-role')))).toStrictEqual(new Set(['text-1']))
  })

  it('judges each role at the floor of its level', () => {
    const wrapper = mount(IdentityRoles)
    const floor   = Object.fromEntries(LEVELS)
    expect(wrapper.findAll('.identity-legend').map(row => row.findAll('span').at(-2)!.text() === 'pass'))
      .toStrictEqual(data.roles.map(role => role.contrast.light.ratio >= floor[role.judged]!))
  })
})

describe('IdentityTypefaces', () => {
  it('reads each role on mount and on resize into an index grouped by face, the pane naming its selector', async () => {
    const wrapper = mount(IdentityTypefaces, { attachTo: document.body })
    await flushPromises()
    expect(wrapper.findAll('[role="tab"]')).toHaveLength(wrapper.findAll('[data-type-role]').length)
    expect(wrapper.text()).toContain('Weight')
    expect(wrapper.get('[data-type-role="h1"]').attributes('data-marked')).toBeDefined()
    const title = wrapper.get<HTMLElement>('[data-type-role="h1"]').element
    title.style.fontSize = '40px'
    globalThis.dispatchEvent(new Event('resize'))
    await flushPromises()
    expect(wrapper.text()).toContain('40px')
    expect(wrapper.text()).toContain('The title takes')
    expect(wrapper.findAll('.identity-copy').at(-1)!.text()).toContain('h1')
    wrapper.unmount()
  })
})

describe('IdentityMarks', () => {
  afterEach(() => { document.body.innerHTML = '' })

  it('names the file that draws the parts turned on', async () => {
    const wrapper  = mount(IdentityMarks)
    const switches = wrapper.findAll('[role="switch"]')
    expect(wrapper.text()).toContain('/title-with-tagline.svg')
    await switches[2]!.trigger('click')
    expect(wrapper.text()).toContain('/title.svg')
    await switches[1]!.trigger('click')
    expect(wrapper.text()).toContain('/logo.svg')
    expect(wrapper.text()).toContain(`viewBox ${data.marks.files['logo.svg'].viewBox}`)
    await switches[0]!.trigger('click')
    expect(wrapper.text()).toContain('Turn a part on')
    await switches[2]!.trigger('click')
    expect(switches[2]!.attributes('aria-checked')).toBe('true')
    expect(wrapper.text()).toContain('No served file draws these parts alone')
    expect(wrapper.text()).toContain('no single file')
  })

  it('sets the mark into a copy of the nav title and ranges its ruler from half the served height to the nav bar, measured again on resize', async () => {
    document.body.innerHTML = '<div class="VPNavBar"><div class="VPNavBarTitle"><a class="title"><img class="logo" data-v-1 /><span>Prose</span></a></div></div>'
    const rect = (height: number) => ({ bottom: height, height, left: 0, right: height, top: 0, width: height }) as DOMRect
    vi.spyOn(document.querySelector('.logo')!, 'getBoundingClientRect').mockReturnValue(rect(24))
    vi.spyOn(document.querySelector('.VPNavBar')!, 'getBoundingClientRect').mockReturnValue(rect(64))
    const wrapper = mount(IdentityMarks, { attachTo: document.body })
    await flushPromises()
    const holder = wrapper.get('.identity-nav [data-v-1]')
    expect(holder.element.tagName).toBe('SPAN')
    expect(holder.html()).toContain('height="24"')
    const thumb = wrapper.get('[role="slider"]')
    expect([thumb.attributes('aria-valuemin'), thumb.attributes('aria-valuemax')]).toStrictEqual(['12', '64'])
    expect(wrapper.get('.identity-diamond').attributes('style')).toBe(`left: ${(24 - 12) / (64 - 12) * 100}%;`)
    wrapper.findComponent(Slider).vm.$emit('change', 40)
    await flushPromises()
    expect(holder.html()).toContain('height="40"')
    expect(wrapper.text()).toContain('40 px tall')
    const name = wrapper.get('.identity-nav .title > span:last-child')
    expect(name.attributes('hidden')).toBeDefined()
    await wrapper.findAll('[role="switch"]')[1]!.trigger('click')
    expect(name.attributes('hidden')).toBeUndefined()
    vi.spyOn(document.querySelector('.VPNavBar')!, 'getBoundingClientRect').mockReturnValue(rect(56))
    globalThis.dispatchEvent(new Event('resize'))
    await flushPromises()
    expect(thumb.attributes('aria-valuemax')).toBe('56')
    wrapper.unmount()
  })
})

describe('IdentityBadges', () => {
  it('draws the chosen badge stroke by stroke and reads out its family', async () => {
    const wrapper = mount(IdentityBadges)
    const strokes = strokesOf(familyEmblem(FAMILY_ORDER[0]).motif).length
    expect(wrapper.text()).toContain(`Stroke 0 of ${strokes}`)
    for (const path of wrapper.findAll('.identity-stroke').filter(each => each.attributes('style')?.includes('--stroke: 0'))) {
      await path.trigger('animationend')
    }
    expect(wrapper.text()).toContain(`Stroke 1 of ${strokes}`)
    expect(wrapper.text()).toContain(data.inks[FAMILY_ORDER[0]].light)
    expect(wrapper.text()).toContain(`${data.ruleCounts[FAMILY_ORDER[0]]} rules`)
    expect(wrapper.text()).toContain(GLOSSARY_FAMILY_META[FAMILY_ORDER[0]].badge)
    await wrapper.findAll('[role="tab"]').at(-1)!.trigger('click')
    expect(wrapper.text()).toContain('Stroke 0 of')
    expect(wrapper.text()).toContain('a concern')
  })
})
