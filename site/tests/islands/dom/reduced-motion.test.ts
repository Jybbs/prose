// @vitest-environment happy-dom
import { describe, expect, vi } from 'vitest'

import { test } from '../../common/support'

const load = async () => {
  vi.resetModules()
  return (await import('../../../src/lib/shared/dom/reduced-motion')).reducedMotion
}

describe('reducedMotion', () => {
  test.for([
    { name: 'matches when the user prefers reduced motion', matches: true,  value: 'reduce' as const },
    { name: 'stays unmatched under no preference',          matches: false, value: 'no-preference' as const }
  ])('$name', async ({ matches, value }, { setReducedMotion }) => {
    setReducedMotion(value)
    expect((await load()).matches).toBe(matches)
  })
})
