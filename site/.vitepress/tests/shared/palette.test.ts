import { converter, formatHex, parse } from 'culori'
import { parse as parseCss }           from 'postcss'

import { PALETTE, paletteCss } from '../../lib/shared/palette'

describe('paletteCss', () => {
  const css   = paletteCss()
  const decls : [string, string][] = []
  parseCss(css).walkDecls(decl => void decls.push([decl.prop, decl.value]))

  it('wraps the declarations in a :root block', () => {
    expect(css.startsWith(':root {\n')).toBe(true)
    expect(css.endsWith('}\n')).toBe(true)
    expect(decls).not.toHaveLength(0)
  })

  it.each(decls)('%s is a concrete hex', (name, value) => {
    expect(formatHex(parse(value))).toBe(value)
  })

  it.each([
    '--prose-palette-ube',
    '--prose-family-cli',
    '--prose-role-accent',
    '--prose-section-usage'
  ])('emits %s under its group prefix', name => {
    expect(decls.map(([prop]) => prop)).toContain(name)
  })
})

describe('PALETTE', () => {
  const lightness = (hex: string): number => converter('oklch')(parse(hex))!.l

  it('blends distinct shares to distinct colors', () => {
    expect(PALETTE['ube-pale']).not.toBe(PALETTE['ube-mid'])
  })

  it.each([
    ['ube-deep',  'darker'],
    ['ube-night', 'darker'],
    ['ube-mid',   'lighter'],
    ['ube-pale',  'lighter']
  ] as const)('mixes %s %s than the base hue', (shade, direction) => {
    const delta = lightness(PALETTE[shade]) - lightness(PALETTE.ube)
    expect(direction === 'darker' ? delta : -delta).toBeLessThan(0)
  })
})
