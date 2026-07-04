// @vitest-environment happy-dom
import { describe, expect, vi } from 'vitest'

import { remeasure } from '../../../src/lib/shared/dom/remeasure'
import { test }      from '../../common/support'

describe('remeasure', () => {
  test('runs measure on each resize and once the fonts settle', async ({ fakeRO, loadFonts }) => {
    const target  = document.createElement('div')
    const measure = vi.fn()

    const observer = remeasure(target, measure)
    expect(measure).not.toHaveBeenCalled()

    fakeRO.resize(target, { width: 10 })
    expect(measure).toHaveBeenCalledTimes(1)

    loadFonts()
    await Promise.resolve()
    expect(measure).toHaveBeenCalledTimes(2)

    expect(observer.disconnect).toBeTypeOf('function')
  })
})
