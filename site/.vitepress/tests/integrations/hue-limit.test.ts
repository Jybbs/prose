import fs from 'node:fs'

import { wcagContrast } from 'culori'

import * as palette    from '../../lib/shared/palette'
import * as vocabulary from '../vocabulary'

const COMPONENTS = fs.globSync(
  '.vitepress/theme/components/integrations/**/*.{css,tsx}',
  { cwd: vocabulary.SITE }
)

const OTHER_HUES = Object.keys(palette.PALETTE).filter(name => !name.startsWith('ube'))
const HUE_CLASS  = new RegExp(`-(?:aside|${OTHER_HUES.join('|')})\\b|--vp-c-(?!bg|brand|divider|gray|text)`)

const inks = (scheme: palette.Scheme) => ({
  link     : palette.SCHEME_ROLES[scheme].link,
  'text-1' : palette.VITEPRESS['text-1'][scheme],
  'text-2' : palette.VITEPRESS['text-2'][scheme],
  ube      : palette.PALETTE.ube
})

const FLOORS = [
  ['a footer, row name, or tab name at rest', 'text-2', 'soft', 4.5],
  ['a hovered, focused, or selected edge',    'ube',    'page', 3.4],
  ['a hovered or focused footer',             'link',   'page', 4.5],
  ['a selected mark',                         'link',   'page', 4.5],
  ['a selected row\'s bar',                   'link',   'soft', 3.5],
  ['a selected row\'s name',                  'text-1', 'soft', 4.5],
  ['a selected tab\'s name',                  'text-1', 'page', 4.5],
  ['a mark in its ring at rest',              'text-2', 'page', 3.1]
] as const

describe('the integration components', () => {
  it.each(COMPONENTS)('%s names no color but a gray or a form of Ube', file => {
    expect(vocabulary.read(file)).not.toMatch(HUE_CLASS)
  })

  it('draws the tool ring in gray', () => {
    const sheet = vocabulary.read('.vitepress/theme/styles/tailwind/base.css')
    expect(sheet).toMatch(/@utility tool-ring \{[^}]*flex-none border-divider bg-bg text-text-2;/)
  })

  describe.each(palette.SCHEMES)('in the %s scheme', scheme => {
    it.each(FLOORS)('clears the contrast floor for %s', (_role, ink, ground, floor) => {
      const contrast = wcagContrast(inks(scheme)[ink], palette.GROUNDS[ground][scheme])
      expect(contrast).toBeGreaterThanOrEqual(floor)
    })
  })
})
