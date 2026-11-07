import { Cascade } from '../../lib/identity/cascade'

describe('Cascade', () => {
  const cascade = Cascade.read([
    ':root { --ground: #fff; --ink: var(--ground); --edge: var(--missing, var(--ink)); }',
    '.dark { --ground: #000; }',
    '@theme static { --accent: #8a80cb; }',
    '@media (min-width: 60rem) { :root { --ground: #f00; } }',
    '.card { --ground: #0f0; }',
    '@font-face { --ground: #00f; }',
    ':root { --ink: color-mix(in srgb, var(--accent) 50%, var(--ground)); }'
  ])

  it('reads a root declaration in both schemes and a dark one in the dark scheme alone', () => {
    expect(cascade.value('--ground')).toStrictEqual({ dark: '#000', light: '#fff' })
  })

  it('reads a theme block in both schemes', () => {
    expect(cascade.value('--accent')).toStrictEqual({ dark: '#8a80cb', light: '#8a80cb' })
  })

  it('lets a later sheet replace an earlier declaration and resolves each reference inside a function', () => {
    expect(cascade.value('--ink')).toStrictEqual({
      dark  : 'color-mix(in srgb, #8a80cb 50%, #000)',
      light : 'color-mix(in srgb, #8a80cb 50%, #fff)'
    })
  })

  it('takes the fallback of a reference to an undeclared property', () => {
    expect(cascade.value('--edge').light).toBe('color-mix(in srgb, #8a80cb 50%, #fff)')
  })

  it('reads no block a media query, a class selector, or another at-rule holds', () => {
    expect(cascade.value('--ground').light).toBe('#fff')
  })
})
