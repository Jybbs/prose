import { familyEmblem } from '../../lib/shared/emblems'
import * as nib         from '../../lib/shared/nib'

describe('nib', () => {
  it.each([[14, 'small'], [21.9, 'small'], [22, 'large'], [96, 'large']] as const)(
    'draws a %s px badge from the %s symbol', (px, size) => {
      expect(nib.nibSize(px)).toBe(size)
    })

  it('holds a stroke and an offset in grid units however far a drawing scales', () => {
    const fit = { scale: 0.8, x: 0, y: 0 }
    expect(nib.perScale(fit, 2)).toBe(2.5)
    expect(nib.diagonal(fit, 0.4)).toBe('translate(0.5 -0.5)')
    expect(nib.fitTransform({ scale: 0.891, x: 1.3, y: 1.325 })).toBe('translate(1.3 1.325) scale(0.891)')
  })

  it.each([
    [undefined,  'aria-hidden="true"'],
    ['Ordering', 'role="img" aria-label="Ordering"']
  ])('writes the badge markup labeled %s', (label, a11y) => {
    expect(nib.nibBadgeHtml(familyEmblem('ordering'), 15.2, label)).toBe(
      `<svg class="nib-badge" width="15.2" height="15.2" viewBox="0 0 24 24" ${a11y}>`
      + '<use href="#nib-harp-small"></use></svg>')
  })
})
