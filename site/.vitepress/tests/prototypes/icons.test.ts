// @vitest-environment happy-dom
import { mount } from '@vue/test-utils'

import Emblem                   from '../../../prototypes/icons/Emblem.vue'
import * as emblems             from '../../../prototypes/icons/emblems'
import IconSpecimen             from '../../../prototypes/icons/IconSpecimen.vue'
import { GLOSSARY_FAMILY_META } from '../../lib/shared/registries'
import { data as platforms }    from '../../lib/usage/platforms.data'
import { rendersAccessibly }    from '../axe'

import type loader from '../../lib/usage/platforms.data'

vi.mock('../../lib/usage/platforms.data', async importOriginal =>
  ({ data: await (await importOriginal<{ default: typeof loader }>()).default.load([]) }))

const newspaper = emblems.EMBLEMS.get(GLOSSARY_FAMILY_META.docs.badge)!

const house = (overrides: Partial<emblems.House> = {}): emblems.House =>
  ({ form: 'line', terminal: 'round', tone: 'mono', weight: 2, ...overrides })

const draw = (setting: emblems.House, standalone = false) =>
  mount(Emblem, { props: { emblem: newspaper, house: setting, standalone } })

const paths = (motif: emblems.Motif): readonly string[] =>
  [...motif.cuts, ...motif.lines, ...motif.shapes]

describe('EMBLEMS', () => {
  it('maps every badge and platform emoji the registries declare and nothing else', () => {
    const declared = new Set([
      ...Object.values(GLOSSARY_FAMILY_META).map(meta => meta.badge),
      ...platforms.map(platform => platform.label.split(' ')[0])
    ])
    expect([...emblems.EMBLEMS.keys()].toSorted()).toStrictEqual([...declared].toSorted())
  })

  it.each([...emblems.EMBLEMS.values()])('draws $motif.label one subpath per path', emblem => {
    for (const path of paths(emblem.motif)) expect(path.match(/m/gi)).toHaveLength(1)
  })
})

describe('isClosed', () => {
  it.each([
    ['M3 11h18',            false],
    ['M10 12h3v4h-3Z',      true],
    ['M10 12h3v4h-3z \n',   true]
  ])('reads %j as closed: %s', (path, expected) => {
    expect(emblems.isClosed(path)).toBe(expected)
  })
})

describe('Emblem', () => {
  it('labels an icon standing alone with its motif', () => {
    const svg = draw(house(), true)
    expect(svg.attributes()).toMatchObject({ 'aria-label': 'Newspaper', role: 'img' })
    expect(svg.attributes('aria-hidden')).toBeUndefined()
  })

  it('hides an icon beside its label from assistive technology', () => {
    const svg = draw(house())
    expect(svg.attributes('aria-hidden')).toBe('true')
    expect(svg.attributes('role')).toBeUndefined()
    expect(svg.attributes('aria-label')).toBeUndefined()
  })

  it('sets the family whose hue a duotone takes', () => {
    const svg = draw(house({ tone: 'duotone' }))
    expect(svg.attributes('data-family')).toBe('docs')
  })

  it.each([
    [house(),                                  { mask: 0, outline: 0, paint: 0, strokes: 6 }],
    [house({ tone: 'duotone' }),               { mask: 1, outline: 0, paint: 1, strokes: 6 }],
    [house({ form: 'fill' }),                  { mask: 1, outline: 1, paint: 1, strokes: 0 }],
    [house({ form: 'fill', tone: 'duotone' }), { mask: 1, outline: 1, paint: 1, strokes: 0 }]
  ])('layers %o as %o', (setting, layers) => {
    const svg = draw(setting)
    expect({
      mask    : svg.findAll('mask').length,
      outline : svg.findAll('.icon-emblem-outline').length,
      paint   : svg.findAll('.icon-emblem-paint').length,
      strokes : svg.findAll('svg > path').length
    }).toStrictEqual(layers)
  })

  it.each([
    ['fill', newspaper.motif.cuts,    'cut', '#000'],
    ['line', paths(newspaper.motif), 'ink', 'currentColor']
  ] as const)('tips only the open strokes of a %s nib', (form, drawn, tip, fill) => {
    const svg    = draw(house({ form, terminal: 'nib' }))
    const tipped = svg.findAll('path[marker-start]')
    const marker = svg.find(`marker[id$="-${tip}"]`)
    expect(svg.findAll('marker')).toHaveLength(2)
    expect(tipped.map(path => path.attributes('d')).toSorted())
      .toStrictEqual(drawn.filter(path => !emblems.isClosed(path)).toSorted())
    for (const path of tipped) {
      expect(path.attributes()).toMatchObject({
        'marker-end'   : `url(#${marker.attributes('id')})`,
        'marker-start' : `url(#${marker.attributes('id')})`
      })
    }
    expect(marker.find('path').attributes('fill')).toBe(fill)
  })

  it('draws a round terminal with no markers', () => {
    const svg = draw(house({ tone: 'duotone' }))
    expect(svg.findAll('marker')).toHaveLength(0)
    expect(svg.findAll('[marker-start]')).toHaveLength(0)
  })
})

describe('IconSpecimen', () => {
  const setting  = house({ form: 'fill', terminal: 'nib', tone: 'duotone' })
  const specimen = () => mount(IconSpecimen, { props: setting })

  it('hides every chip and tile icon from assistive technology', () => {
    const hidden = specimen().findAll('.family-chip svg, .rule-card-badge svg')
    expect(hidden.map(svg => svg.attributes('aria-hidden')))
      .toStrictEqual(Array.from({ length: 2 * emblems.EMBLEMS.size }, () => 'true'))
  })

  it('labels every card-size icon with its motif', () => {
    const card = specimen().findAll('.icon-specimen-card svg')
    expect(card.map(svg => [svg.attributes('role'), svg.attributes('aria-label')]))
      .toStrictEqual([...emblems.EMBLEMS.values()].map(emblem => ['img', emblem.motif.label]))
  })

  it('lists the house style it draws', () => {
    expect(specimen().findAll('.icon-specimen-axes dd').map(dd => dd.text()))
      .toStrictEqual(['fill', 'nib', 'duotone', '2'])
  })

  rendersAccessibly(() => specimen().html())
})

