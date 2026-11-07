// @vitest-environment happy-dom
import { flushPromises, mount } from '@vue/test-utils'

import { IdentityBadges }                     from '../../theme/components/identity/IdentityBadges'
import { IdentityMarks }                      from '../../theme/components/identity/IdentityMarks'
import { IdentityPalette }                    from '../../theme/components/identity/IdentityPalette'
import { IdentityRoles }                      from '../../theme/components/identity/IdentityRoles'
import { IdentityTypefaces }                  from '../../theme/components/identity/IdentityTypefaces'
import loader, { data }                       from '../../lib/identity/identity.data'
import { ROLES }                              from '../../lib/identity/roles'
import { familyEmblem, strokesOf }            from '../../lib/shared/emblems'
import { FAMILY_ORDER, GLOSSARY_FAMILY_META } from '../../lib/shared/registries'

vi.mock('../../lib/identity/identity.data', async importOriginal => {
  const actual = await importOriginal<typeof import('../../lib/identity/identity.data')>()
  return { ...actual, data: await actual.default.load([]) }
})

describe('the identity data', () => {
  it('resolves every role to a color in each scheme', () => {
    expect(loader.watch).toBeDefined()
    expect(ROLES.flatMap(role => Object.values(data.roles[role.id])).filter(value => !/^#[\da-f]{6}$/.test(value))).toStrictEqual([])
  })

  it('reads a nib ink in each scheme for every family and a rule count for every rule family', () => {
    expect(Object.keys(data.inks).toSorted()).toStrictEqual(Object.keys(GLOSSARY_FAMILY_META).toSorted())
    expect(Object.keys(data.ruleCounts).toSorted()).toStrictEqual([...FAMILY_ORDER].toSorted())
  })
})

describe('IdentityPalette', () => {
  it('names the strip the reader chooses beneath the band with its values and contrast', async () => {
    const wrapper = mount(IdentityPalette)
    const strips  = wrapper.findAll('.identity-strip')
    expect(strips).toHaveLength(data.swatches.length)
    await strips.at(-1)!.trigger('click')
    await flushPromises()
    const last = data.swatches.at(-1)!
    expect(wrapper.text()).toContain(last.title)
    expect(wrapper.text()).toContain(last.values.dark)
    expect(wrapper.text()).toContain(`${last.contrast.light.ratio.toFixed(2)}:1`)
    expect(wrapper.findAll('button.copy').length).toBeGreaterThanOrEqual(3)
  })
})

describe('IdentityRoles', () => {
  it('sets the passage in the chosen scheme and marks each place it uses the chosen role', async () => {
    const wrapper = mount(IdentityRoles)
    const passage = () => wrapper.get('[data-role="page"]').attributes('style')
    expect(passage()).toContain(`--vp-c-bg: ${data.roles.page.light}`)
    await wrapper.findAll('.toggle-pill').find(pill => pill.text() === 'Dark')!.trigger('click')
    expect(passage()).toContain(`--vp-c-bg: ${data.roles.page.dark}`)
    const legend = wrapper.findAll('.identity-legend')
    await legend[ROLES.findIndex(role => role.id === 'code-ink')]!.trigger('click')
    const marked = wrapper.findAll('[data-marked]')
    expect(marked.length).toBeGreaterThan(1)
    expect(new Set(marked.map(each => each.attributes('data-role')))).toStrictEqual(new Set(['code-ink']))
  })
})

describe('IdentityTypefaces', () => {
  it('reads each role on mount and on resize into an index grouped by face', async () => {
    const wrapper = mount(IdentityTypefaces, { attachTo: document.body })
    await flushPromises()
    expect(wrapper.findAll('[role="tab"]')).toHaveLength(wrapper.findAll('[data-type-role]').length)
    expect(wrapper.text()).toContain('Weight')
    const title = wrapper.get<HTMLElement>('[data-type-role="h1"]').element
    title.style.fontSize = '40px'
    globalThis.dispatchEvent(new Event('resize'))
    await flushPromises()
    expect(wrapper.text()).toContain('40px')
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
    await switches[0]!.trigger('click')
    expect(wrapper.text()).toContain('Turn a part on')
    await switches[2]!.trigger('click')
    expect(switches[2]!.attributes('aria-checked')).toBe('true')
    expect(wrapper.text()).toContain('No served file draws these parts alone')
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
    expect(wrapper.text()).toContain(String(data.ruleCounts[FAMILY_ORDER[0]]))
    await wrapper.findAll('[role="tab"]').at(-1)!.trigger('click')
    expect(wrapper.text()).toContain('Stroke 0 of')
    expect(wrapper.text()).toContain('A concern rather than a rule family')
  })
})
