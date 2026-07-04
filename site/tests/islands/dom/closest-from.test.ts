// @vitest-environment happy-dom
import { beforeEach, describe, expect, test } from 'vitest'

import { closestFrom } from '../../../src/lib/shared/dom/closest-from'

describe('closestFrom', () => {
  beforeEach(() => {
    document.body.innerHTML = '<section class="s"><button class="b"></button></section>'
  })

  test.each([
    { name: 'resolves the target itself',    match: 'b', selector: '.b', targetSel: '.b' },
    { name: 'climbs to a matching ancestor', match: 's', selector: '.s', targetSel: '.b' }
  ])('$name', ({ match, selector, targetSel }) => {
    const target = document.querySelector(targetSel)
    expect(closestFrom({ target } as unknown as Event, selector)).toHaveClass(match)
  })

  test.each([
    { name: 'returns null when nothing matches',       selector: '.none', target: () => document.querySelector('.b') },
    { name: 'returns null when the target is document', selector: '.b',   target: () => document },
    { name: 'returns null when the target is null',     selector: '.b',   target: () => null }
  ])('$name', ({ selector, target }) => {
    expect(closestFrom({ target: target() } as unknown as Event, selector)).toBeNull()
  })
})
